//! Grid assembly: split coefficient products into low/high words and
//! distribute them across output columns.

use crate::bits_ext::mask;
use crate::error::Error;

/// Compute all `K * K` coefficient products, returning a flat
/// vector indexed by `i * k + j` where `k == coefficients.len()`.
///
/// Each product is the full `2N`-bit coefficient product, masked
/// to `2N` bits.  Callers typically pass the coefficient multiplier
/// they already have (e.g. the native `u128` multiply used here, or
/// a booth-tree-based one).
///
/// # Errors
///
/// Returns [`Error::CoefficientCountMismatch`] if `a` and `b` have
/// different lengths, or [`Error::ZeroCoefficientCount`] if either
/// is empty.
pub fn element_products(
    a: &[u128],
    b: &[u128],
    coefficient_width: usize,
) -> Result<Vec<u128>, Error> {
    match (a.len(), b.len()) {
        (0, _) | (_, 0) => Err(Error::ZeroCoefficientCount),
        (x, y) if x != y => Err(Error::CoefficientCountMismatch {
            a_count: x,
            b_count: y,
        }),
        _ => {
            let m = mask(2 * coefficient_width);
            Ok(a.iter()
                .flat_map(|ai| b.iter().map(move |bj| ai.wrapping_mul(*bj) & m))
                .collect())
        }
    }
}

/// Distribute element products across the `2K` output columns.
///
/// Each product of width `2N` is split into its low `word_len` bits
/// (which feed column `i + j`) and its high `2N - word_len` bits
/// (which feed column `i + j + 1`).  The inner `Vec<u128>` of
/// column `c` lists every contributing term at that column.
///
/// # Errors
///
/// Returns [`Error::WordLengthTooLarge`] if `word_len >= 2 *
/// coefficient_width`.
pub fn assemble_columns(
    products: &[u128],
    num_coefficients: usize,
    coefficient_width: usize,
    word_len: usize,
) -> Result<Vec<Vec<u128>>, Error> {
    let product_width = 2 * coefficient_width;
    if word_len >= product_width {
        Err(Error::WordLengthTooLarge {
            word_len,
            product_width,
        })
    } else {
        let column_count = 2 * num_coefficients;
        let low_mask = mask(word_len);
        let columns: Vec<Vec<u128>> = (0..column_count)
            .map(|c| {
                let low_contrib = low_words_into_column(products, num_coefficients, low_mask, c);
                let high_contrib =
                    high_words_into_column(products, num_coefficients, word_len, c);
                low_contrib.chain(high_contrib).collect()
            })
            .collect();
        Ok(columns)
    }
}

/// Iterator over low-word contributions to column `c`.
fn low_words_into_column(
    products: &[u128],
    k: usize,
    low_mask: u128,
    c: usize,
) -> impl Iterator<Item = u128> + '_ {
    let lower = c.saturating_sub(k.saturating_sub(1));
    let upper = (c + 1).min(k);
    (lower..upper).map(move |i| {
        let j = c - i;
        products[i * k + j] & low_mask
    })
}

/// Iterator over high-word contributions to column `c`.
fn high_words_into_column(
    products: &[u128],
    k: usize,
    word_len: usize,
    c: usize,
) -> impl Iterator<Item = u128> + '_ {
    if c == 0 {
        Box::new(core::iter::empty()) as Box<dyn Iterator<Item = u128>>
    } else {
        let target = c - 1;
        let lower = target.saturating_sub(k.saturating_sub(1));
        let upper = (target + 1).min(k);
        Box::new((lower..upper).map(move |i| {
            let j = target - i;
            products[i * k + j] >> word_len
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_products_computes_all_pairs() -> Result<(), Error> {
        let a = [1_u128, 2, 3];
        let b = [4_u128, 5, 6];
        let products = element_products(&a, &b, 8)?;
        assert_eq!(products.len(), 9);
        assert_eq!(products[0], 4);
        assert_eq!(products[1], 5);
        assert_eq!(products[4], 10);
        Ok(())
    }

    #[test]
    fn element_products_rejects_mismatched_lengths() {
        let a = [1_u128, 2];
        let b = [4_u128, 5, 6];
        assert!(element_products(&a, &b, 8).is_err());
    }

    #[test]
    fn element_products_rejects_empty() {
        let a: [u128; 0] = [];
        let b: [u128; 0] = [];
        assert!(element_products(&a, &b, 8).is_err());
    }

    #[test]
    fn assemble_columns_has_correct_column_count() -> Result<(), Error> {
        let a = [1_u128, 2, 3];
        let b = [4_u128, 5, 6];
        let products = element_products(&a, &b, 8)?;
        let columns = assemble_columns(&products, 3, 8, 8)?;
        assert_eq!(columns.len(), 6);
        Ok(())
    }

    #[test]
    fn assemble_columns_rejects_oversize_word_len() {
        let products = vec![1_u128, 2, 3, 4];
        assert!(assemble_columns(&products, 2, 8, 16).is_err());
    }

    #[test]
    fn assemble_columns_sum_equals_polynomial_product() -> Result<(), Error> {
        let a = [3_u128, 1, 4];
        let b = [2_u128, 7, 1];
        let products = element_products(&a, &b, 8)?;
        let columns = assemble_columns(&products, 3, 8, 8)?;
        let column_sums: Vec<u128> = columns
            .iter()
            .map(|col| col.iter().fold(0_u128, |acc, t| acc.wrapping_add(*t)))
            .collect();
        let recomposed: u128 = column_sums
            .iter()
            .enumerate()
            .fold(0_u128, |acc, (c, s)| acc.wrapping_add(*s << (c * 8)));
        let reference: u128 = (0..3)
            .flat_map(|i| (0..3).map(move |j| (i, j)))
            .fold(0_u128, |acc, (i, j)| acc + a[i] * b[j] * (1_u128 << ((i + j) * 8)));
        assert_eq!(recomposed & ((1_u128 << 48) - 1), reference);
        Ok(())
    }
}
