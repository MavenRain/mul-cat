//! Conversion helpers between [`rhdl_bits::Bits`] and `u128`.
//!
//! All internal arithmetic in this crate is carried out in `u128`
//! for simplicity, with conversions confined to the boundary.  The
//! underlying representation of [`Bits<N>`](rhdl_bits::Bits) is a
//! `u128`, so these helpers are essentially zero-cost wrappers over
//! [`Bits::raw`](rhdl_bits::Bits::raw) and
//! [`bits_masked`].

use rhdl_bits::{BitWidth, Bits, W, bits_masked};

/// The `u128` mask for an `N`-bit value.
///
/// Returns `u128::MAX` when `N >= 128`.
#[must_use]
pub const fn mask(n: usize) -> u128 {
    match n {
        0 => 0,
        128.. => u128::MAX,
        _ => (1_u128 << n) - 1,
    }
}

/// Extract the underlying `u128` from a [`Bits<N>`] value.
///
/// # Examples
///
/// ```
/// use mul_cat::bits_ext::to_u128;
/// use rhdl_bits::bits;
///
/// assert_eq!(to_u128(bits::<8>(0xAB)), 0xAB);
/// ```
#[must_use]
pub const fn to_u128<const N: usize>(b: Bits<N>) -> u128
where
    W<N>: BitWidth,
{
    b.raw()
}

/// Construct a [`Bits<N>`] from a `u128`, masking excess bits.
///
/// Unlike [`Bits::<N>::from`](rhdl_bits::Bits), this function will
/// not panic when the input exceeds the `N`-bit mask; it masks the
/// value first.
///
/// # Examples
///
/// ```
/// use mul_cat::bits_ext::from_u128;
/// use mul_cat::bits_ext::to_u128;
/// use rhdl_bits::Bits;
///
/// let b: Bits<8> = from_u128(0x1AB);
/// assert_eq!(to_u128(b), 0xAB);
/// ```
#[must_use]
pub const fn from_u128<const N: usize>(value: u128) -> Bits<N>
where
    W<N>: BitWidth,
{
    bits_masked::<N>(value)
}

/// Return true if the `i`-th bit of `b` is set.
///
/// Returns `false` when `i >= N`, matching the "pad with zeros"
/// convention needed for Booth-window extraction.
#[must_use]
pub const fn bit_at<const N: usize>(b: Bits<N>, i: usize) -> bool
where
    W<N>: BitWidth,
{
    if i < N {
        (b.raw() >> i) & 1 != 0
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhdl_bits::bits;

    #[test]
    fn mask_small_values() {
        assert_eq!(mask(0), 0);
        assert_eq!(mask(1), 1);
        assert_eq!(mask(8), 0xFF);
        assert_eq!(mask(17), 0x1_FFFF);
        assert_eq!(mask(64), u128::from(u64::MAX));
    }

    #[test]
    fn mask_at_limit_is_u128_max() {
        assert_eq!(mask(128), u128::MAX);
    }

    #[test]
    fn to_u128_round_trip() {
        let values: [u128; 5] = [0, 1, 0xFF, 0x1234_5678, 0xDEAD_BEEF_CAFE];
        values.iter().for_each(|&v| {
            let b: Bits<64> = from_u128(v);
            assert_eq!(to_u128(b), v);
        });
    }

    #[test]
    fn from_u128_masks_excess_bits() {
        let b: Bits<4> = from_u128(0xAB);
        assert_eq!(to_u128(b), 0xB);
    }

    #[test]
    fn to_u128_extracts_all_bits() {
        assert_eq!(to_u128(bits::<8>(0xAB)), 0xAB);
        assert_eq!(to_u128(bits::<17>(0x1_FFFF)), 0x1_FFFF);
    }

    #[test]
    fn bit_at_reads_individual_bits() {
        let b = bits::<8>(0b1010_0101);
        assert!(bit_at(b, 0));
        assert!(!bit_at(b, 1));
        assert!(bit_at(b, 2));
        assert!(!bit_at(b, 3));
        assert!(bit_at(b, 5));
        assert!(bit_at(b, 7));
    }

    #[test]
    fn bit_at_past_width_returns_false() {
        let b = bits::<8>(0xFF);
        assert!(!bit_at(b, 8));
        assert!(!bit_at(b, 100));
    }
}
