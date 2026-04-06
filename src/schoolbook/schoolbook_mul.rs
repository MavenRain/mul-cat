//! Schoolbook polynomial multiplication with per-column reduction.

use crate::bits_ext::{mask, to_u128};
use crate::carry_save::CarrySavePair;
use crate::error::Error;
use crate::evaluate::tree_stage::reduce_terms;
use crate::schoolbook::grid::{assemble_columns, element_products};
use crate::topology::Topology;
use hdl_cat_bits::Bits;

/// Result of a schoolbook multiplication: one carry-save pair per
/// output column, resolvable against the column bit width.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct SchoolbookResult {
    columns: Vec<CarrySavePair>,
    column_width: usize,
}

impl SchoolbookResult {
    /// The per-column carry-save pairs.
    pub fn columns(&self) -> &[CarrySavePair] {
        &self.columns
    }

    /// The operating bit width of every column's carry-save pair.
    #[must_use]
    pub const fn column_width(&self) -> usize {
        self.column_width
    }

    /// Resolve every column into a single `u128` value.
    #[must_use]
    pub fn resolve_columns(&self) -> Vec<u128> {
        let m = mask(self.column_width);
        self.columns.iter().map(|p| p.resolve(m)).collect()
    }
}

/// Compute the schoolbook polynomial product of two coefficient
/// arrays, reducing each output column via the supplied topology.
///
/// The coefficients are of width `N` bits; each product is of
/// width `2N`; the split word length is `word_len`.  The column
/// operating width must accommodate any single contribution, which
/// is at most `2N - word_len` bits.  For safety and parity with the
/// Supranational RTL we reduce each column at width `2N`.
///
/// # Errors
///
/// - [`Error::CoefficientCountMismatch`] if `a` and `b` differ in length.
/// - [`Error::ZeroCoefficientCount`] if either is empty.
/// - [`Error::ZeroBitWidth`] if `N == 0`.
/// - [`Error::BitWidthTooLarge`] if `2N > 128`.
/// - [`Error::WordLengthTooLarge`] if `word_len >= 2N`.
/// - Any error from the reduction topology.
///
/// # Examples
///
/// ```
/// use mul_cat::schoolbook::schoolbook_mul::schoolbook_multiply;
/// use mul_cat::topology::wallace::Wallace;
/// use hdl_cat_bits::Bits;
///
/// let a = [Bits::<8>::new_wrapping(3), Bits::<8>::new_wrapping(1), Bits::<8>::new_wrapping(4)];
/// let b = [Bits::<8>::new_wrapping(2), Bits::<8>::new_wrapping(7), Bits::<8>::new_wrapping(1)];
/// let column_count = schoolbook_multiply::<8>(&a, &b, 4, &Wallace)
///     .map(|r| r.columns().len())
///     .ok();
/// assert_eq!(column_count, Some(6));
/// ```
pub fn schoolbook_multiply<const N: usize>(
    a: &[Bits<N>],
    b: &[Bits<N>],
    word_len: usize,
    topology: &impl Topology,
) -> Result<SchoolbookResult, Error> {
    validate_dimensions::<N>(a, b, word_len)?;
    let a_vals: Vec<u128> = a.iter().copied().map(to_u128).collect();
    let b_vals: Vec<u128> = b.iter().copied().map(to_u128).collect();
    let products = element_products(&a_vals, &b_vals, N)?;
    let columns = assemble_columns(&products, a.len(), N, word_len)?;
    let column_width = 2 * N;
    let reduced: Vec<CarrySavePair> = columns
        .iter()
        .map(|terms| reduce_terms(topology, terms, N))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SchoolbookResult {
        columns: reduced,
        column_width,
    })
}

fn validate_dimensions<const N: usize>(
    a: &[impl Sized],
    b: &[impl Sized],
    word_len: usize,
) -> Result<(), Error> {
    if N == 0 {
        Err(Error::ZeroBitWidth)?;
    }
    let product_width = 2 * N;
    if product_width > 128 {
        Err(Error::BitWidthTooLarge { width: N, max: 64 })?;
    }
    if word_len >= product_width {
        Err(Error::WordLengthTooLarge {
            word_len,
            product_width,
        })?;
    }
    if a.len() != b.len() {
        Err(Error::CoefficientCountMismatch {
            a_count: a.len(),
            b_count: b.len(),
        })?;
    }
    if a.is_empty() {
        Err(Error::ZeroCoefficientCount)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::wallace::Wallace;
    use proptest::prelude::*;

    /// Evaluate the polynomial with coefficients `coeffs` at `base`.
    fn eval_at_base(coeffs: &[u128], base: u128) -> u128 {
        coeffs
            .iter()
            .rev()
            .fold(0_u128, |acc, c| acc.wrapping_mul(base).wrapping_add(*c))
    }

    /// Interpret column values as digits in the given base and
    /// reassemble into a single `u128`.
    fn recombine_columns(column_values: &[u128], base: u128) -> u128 {
        column_values
            .iter()
            .rev()
            .fold(0_u128, |acc, v| acc.wrapping_mul(base).wrapping_add(*v))
    }

    #[test]
    fn schoolbook_preserves_polynomial_value_at_base() -> Result<(), Error> {
        let a = [Bits::<8>::new_wrapping(3), Bits::<8>::new_wrapping(1), Bits::<8>::new_wrapping(4)];
        let b = [Bits::<8>::new_wrapping(2), Bits::<8>::new_wrapping(7), Bits::<8>::new_wrapping(1)];
        let word_len = 4;
        let base: u128 = 1 << word_len;
        let result = schoolbook_multiply::<8>(&a, &b, word_len, &Wallace)?;
        let column_values = result.resolve_columns();
        let a_vals: Vec<u128> = a.iter().copied().map(to_u128).collect();
        let b_vals: Vec<u128> = b.iter().copied().map(to_u128).collect();
        let expected = eval_at_base(&a_vals, base).wrapping_mul(eval_at_base(&b_vals, base));
        let recombined = recombine_columns(&column_values, base);
        assert_eq!(recombined, expected);
        Ok(())
    }

    #[test]
    fn schoolbook_rejects_mismatched_lengths() {
        let a = [Bits::<8>::new_wrapping(1), Bits::<8>::new_wrapping(2)];
        let b = [Bits::<8>::new_wrapping(3), Bits::<8>::new_wrapping(4), Bits::<8>::new_wrapping(5)];
        assert!(schoolbook_multiply::<8>(&a, &b, 4, &Wallace).is_err());
    }

    #[test]
    fn schoolbook_rejects_empty_input() {
        let a: [Bits<8>; 0] = [];
        let b: [Bits<8>; 0] = [];
        assert!(schoolbook_multiply::<8>(&a, &b, 4, &Wallace).is_err());
    }

    #[test]
    fn schoolbook_rejects_oversize_word_len() {
        let a = [Bits::<8>::new_wrapping(1)];
        let b = [Bits::<8>::new_wrapping(2)];
        assert!(schoolbook_multiply::<8>(&a, &b, 16, &Wallace).is_err());
    }

    proptest! {
        #[test]
        fn column_sums_recombine_to_polynomial_product(
            a in proptest::collection::vec(0_u128..16, 1..5),
            b_seed in proptest::collection::vec(0_u128..16, 1..5),
        ) {
            let k = a.len().min(b_seed.len());
            let a = &a[..k];
            let b = &b_seed[..k];
            let a_bits: Vec<Bits<8>> = a.iter().map(|v| Bits::<8>::new_wrapping(*v)).collect();
            let b_bits: Vec<Bits<8>> = b.iter().map(|v| Bits::<8>::new_wrapping(*v)).collect();
            let word_len = 4;
            let base: u128 = 1 << word_len;
            let result = schoolbook_multiply::<8>(&a_bits, &b_bits, word_len, &Wallace).ok();
            prop_assert!(result.is_some());
            let values = result.map(|r| r.resolve_columns()).unwrap_or_default();
            let expected = eval_at_base(a, base).wrapping_mul(eval_at_base(b, base));
            let recombined = recombine_columns(&values, base);
            let column_width_mask = mask(2 * 8 * k.max(1));
            prop_assert_eq!(recombined & column_width_mask, expected & column_width_mask);
        }
    }
}
