//! Booth partial product generation.
//!
//! Given a Booth digit and an `N`-bit unsigned multiplicand, produce
//! the corresponding sign-extended, shifted partial product as a
//! `u128` value masked to `2N` bits.  The sum (modulo `2^{2N}`) of
//! all partial products equals `A * B` because the true product of
//! two `N`-bit unsigned values fits in `2N` bits.

use crate::bits_ext::{mask, to_u128};
use crate::booth::digit::BoothDigit;
use hdl_cat_bits::Bits;

/// Compute the sign-extended partial product for a single Booth digit.
///
/// The result is `digit * multiplicand` reduced modulo `2^{2N}`,
/// i.e., sign-extended to the output width as two's complement.
/// This does not include the weight shift for the digit index; see
/// [`shifted_partial_product`].
#[must_use]
pub fn partial_product<const N: usize>(multiplicand: Bits<N>, digit: BoothDigit) -> u128 {
    let m = mask(2 * N);
    let a = to_u128(multiplicand);
    match digit {
        BoothDigit::Zero => 0,
        BoothDigit::PlusOne => a & m,
        BoothDigit::PlusTwo => (a << 1) & m,
        BoothDigit::MinusOne => (!a).wrapping_add(1) & m,
        BoothDigit::MinusTwo => (!(a << 1)).wrapping_add(1) & m,
    }
}

/// Compute the partial product for digit `i`, including the weight
/// shift by `2 * i` bits.
///
/// The result is masked to `2N` bits.  Summing all shifted partial
/// products modulo `2^{2N}` yields the full product.
#[must_use]
pub fn shifted_partial_product<const N: usize>(
    multiplicand: Bits<N>,
    digit: BoothDigit,
    digit_index: usize,
) -> u128 {
    let m = mask(2 * N);
    (partial_product(multiplicand, digit) << (2 * digit_index)) & m
}

/// Produce the full array of shifted partial products for every
/// digit position, in ascending weight order.
///
/// # Examples
///
/// ```
/// use mul_cat::booth::digit::encode_all;
/// use mul_cat::booth::partial_product::all_shifted_partial_products;
/// use hdl_cat_bits::Bits;
///
/// let a = Bits::<17>::new_wrapping(12345);
/// let b = Bits::<17>::new_wrapping(6789);
/// let digits = encode_all(b);
/// let partials = all_shifted_partial_products(a, &digits);
/// let mask: u128 = (1_u128 << 34) - 1;
/// let sum: u128 = partials.iter().fold(0_u128, |acc, p| acc.wrapping_add(*p)) & mask;
/// assert_eq!(sum, 12345_u128 * 6789);
/// ```
#[must_use]
pub fn all_shifted_partial_products<const N: usize>(
    multiplicand: Bits<N>,
    digits: &[BoothDigit],
) -> Vec<u128> {
    digits
        .iter()
        .enumerate()
        .map(|(i, d)| shifted_partial_product(multiplicand, *d, i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::booth::digit::encode_all;
    use proptest::prelude::*;

    #[test]
    fn zero_digit_produces_zero_partial() {
        let a = Bits::<17>::new_wrapping(12345);
        assert_eq!(partial_product(a, BoothDigit::Zero), 0);
    }

    #[test]
    fn plus_one_returns_multiplicand() {
        let a = Bits::<17>::new_wrapping(12345);
        assert_eq!(partial_product(a, BoothDigit::PlusOne), 12345);
    }

    #[test]
    fn plus_two_returns_doubled_multiplicand() {
        let a = Bits::<17>::new_wrapping(12345);
        assert_eq!(partial_product(a, BoothDigit::PlusTwo), 24690);
    }

    #[test]
    fn minus_one_returns_twos_complement() {
        let a = Bits::<17>::new_wrapping(1);
        let mask: u128 = (1_u128 << 34) - 1;
        assert_eq!(partial_product(a, BoothDigit::MinusOne), mask);
    }

    #[test]
    fn minus_two_returns_twos_complement_doubled() {
        let a = Bits::<17>::new_wrapping(1);
        let expected = ((1_u128 << 34) - 2) & ((1_u128 << 34) - 1);
        assert_eq!(partial_product(a, BoothDigit::MinusTwo), expected);
    }

    proptest! {
        #[test]
        fn shifted_partial_products_sum_to_product(
            a in 0_u128..(1 << 17),
            b in 0_u128..(1 << 17),
        ) {
            let aa = Bits::<17>::new_wrapping(a);
            let bb = Bits::<17>::new_wrapping(b);
            let digits = encode_all(bb);
            let partials = all_shifted_partial_products(aa, &digits);
            let m: u128 = (1_u128 << 34) - 1;
            let sum = partials.iter().fold(0_u128, |acc, p| acc.wrapping_add(*p)) & m;
            prop_assert_eq!(sum, a * b);
        }
    }
}
