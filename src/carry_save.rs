//! Carry-save pair and the 3-to-2 compressor.
//!
//! A carry-save pair holds two `u128` values such that their arithmetic
//! sum (modulo `2^width`) equals the intended numerical value.  A
//! 3-to-2 compressor (full adder replicated across all bit positions)
//! reduces three input terms to one carry-save pair per bit position,
//! with the carry output shifted left by one so the pair is ready
//! for the next level of reduction.

/// A carry-save representation of a value: `carry + sum` equals the
/// represented value (modulo the operating bit width).
///
/// The `carry` component already incorporates the weight shift by one
/// produced by a carry-save adder, so callers can simply add the two
/// fields to obtain the final value.
///
/// # Examples
///
/// ```
/// use mul_cat::carry_save::CarrySavePair;
///
/// let pair = CarrySavePair::new(0b0010, 0b0101);
/// assert_eq!(pair.resolve(0xFF), 0b0111);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct CarrySavePair {
    carry: u128,
    sum: u128,
}

impl CarrySavePair {
    /// Construct a carry-save pair directly.
    pub const fn new(carry: u128, sum: u128) -> Self {
        Self { carry, sum }
    }

    /// The carry component (already shifted left by one).
    #[must_use]
    pub const fn carry(self) -> u128 {
        self.carry
    }

    /// The sum component.
    #[must_use]
    pub const fn sum(self) -> u128 {
        self.sum
    }

    /// Resolve the pair into its full numerical value, masked to the
    /// given width.
    #[must_use]
    pub const fn resolve(self, mask: u128) -> u128 {
        (self.carry.wrapping_add(self.sum)) & mask
    }

    /// The zero carry-save pair.
    pub const fn zero() -> Self {
        Self { carry: 0, sum: 0 }
    }
}

/// Apply a bitwise 3-to-2 compressor to three input terms.
///
/// For each bit position: `sum[i] = a[i] ^ b[i] ^ c[i]`,
/// `raw_carry[i] = majority(a[i], b[i], c[i])`.  The returned pair
/// has the carry left-shifted by one so that it carries the correct
/// weight for the next reduction level.  Both outputs are masked to
/// the given width.
///
/// # Examples
///
/// ```
/// use mul_cat::carry_save::compress_three;
///
/// let pair = compress_three(0b1100, 0b1010, 0b0110, 0xFF);
/// // sum = 0000, carry before shift = 1110, carry after shift = 11100
/// assert_eq!(pair.sum(), 0b0000);
/// assert_eq!(pair.carry(), 0b11100);
/// assert_eq!(pair.resolve(0xFF), 0b11100);
/// ```
pub const fn compress_three(a: u128, b: u128, c: u128, mask: u128) -> CarrySavePair {
    let sum = (a ^ b ^ c) & mask;
    let raw_carry = (a & b) | (a & c) | (b & c);
    let carry = (raw_carry << 1) & mask;
    CarrySavePair::new(carry, sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn compressor_on_three_ones_gives_one_one() {
        let pair = compress_three(1, 1, 1, 0xFF);
        assert_eq!(pair.sum(), 1);
        assert_eq!(pair.carry(), 0b10);
        assert_eq!(pair.resolve(0xFF), 3);
    }

    #[test]
    fn compressor_on_three_zeros_gives_zero() {
        let pair = compress_three(0, 0, 0, 0xFF);
        assert_eq!(pair.sum(), 0);
        assert_eq!(pair.carry(), 0);
        assert_eq!(pair.resolve(0xFF), 0);
    }

    #[test]
    fn zero_pair_is_zero() {
        assert_eq!(CarrySavePair::zero().resolve(0xFF), 0);
    }

    proptest! {
        #[test]
        fn compressor_preserves_sum_modulo_mask(
            a in any::<u64>(),
            b in any::<u64>(),
            c in any::<u64>(),
        ) {
            let mask: u128 = (1_u128 << 66) - 1;
            let (aa, bb, cc) = (u128::from(a), u128::from(b), u128::from(c));
            let pair = compress_three(aa, bb, cc, mask);
            let expected = (aa + bb + cc) & mask;
            prop_assert_eq!(pair.resolve(mask), expected);
        }
    }
}
