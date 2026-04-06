//! Booth encoding stage: operands to partial products.
//!
//! The analogue of the GP stage in `cpa-cat`: this pure function
//! takes the operand bits and produces the sequence of initial
//! terms that the reduction tree will consume.

use crate::booth::digit::encode_all;
use crate::booth::partial_product::all_shifted_partial_products;
use hdl_cat_bits::Bits;

/// Produce the list of shifted Booth partial products for `a * b`.
///
/// The result has `ceil((N + 1) / 2)` entries, each of them a
/// `u128` value masked to `2N` bits.  Their (wrapping) sum equals
/// `a * b`.
///
/// # Examples
///
/// ```
/// use mul_cat::evaluate::booth_stage::booth_partial_products;
/// use hdl_cat_bits::Bits;
///
/// let pp = booth_partial_products(Bits::<17>::new_wrapping(12345), Bits::<17>::new_wrapping(6789));
/// let mask: u128 = (1_u128 << 34) - 1;
/// let sum = pp.iter().fold(0_u128, |acc, p| acc.wrapping_add(*p)) & mask;
/// assert_eq!(sum, 12345_u128 * 6789);
/// ```
#[must_use]
pub fn booth_partial_products<const N: usize>(a: Bits<N>, b: Bits<N>) -> Vec<u128> {
    let digits = encode_all(b);
    all_shifted_partial_products(a, &digits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn partial_products_sum_to_product(
            a in 0_u128..(1 << 17),
            b in 0_u128..(1 << 17),
        ) {
            let pp = booth_partial_products(Bits::<17>::new_wrapping(a), Bits::<17>::new_wrapping(b));
            let m: u128 = (1_u128 << 34) - 1;
            let sum = pp.iter().fold(0_u128, |acc, p| acc.wrapping_add(*p)) & m;
            prop_assert_eq!(sum, a * b);
        }
    }

    #[test]
    fn partial_products_have_expected_count() {
        let pp = booth_partial_products(Bits::<17>::new_wrapping(1), Bits::<17>::new_wrapping(1));
        assert_eq!(pp.len(), 9);
    }
}
