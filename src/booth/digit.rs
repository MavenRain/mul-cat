//! Radix-4 Booth digit: a signed coefficient in `{-2, -1, 0, +1, +2}`.
//!
//! The N-bit unsigned multiplier `B` is decomposed into `ceil((N+1)/2)`
//! overlapping 3-bit windows.  Window `i` reads bits at positions
//! `2i - 1`, `2i`, and `2i + 1`, with bits outside `[0, N)` treated
//! as zero.  Each window is recoded to a signed radix-4 digit
//! according to the standard Booth table.

use crate::bits_ext::bit_at;
use rhdl_bits::{BitWidth, Bits, W};

/// A signed radix-4 Booth digit.
///
/// The value of a digit is its signed integer: zero, plus or minus
/// one, or plus or minus two.  A Booth-encoded multiplier represents
/// `B` as `sum d_i * 4^i` where each `d_i` is one of these digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub enum BoothDigit {
    /// `0 * multiplicand`.
    Zero,
    /// `+1 * multiplicand`.
    PlusOne,
    /// `+2 * multiplicand`.
    PlusTwo,
    /// `-2 * multiplicand`.
    MinusTwo,
    /// `-1 * multiplicand`.
    MinusOne,
}

impl BoothDigit {
    /// Decode a raw 3-bit window `[high, mid, low]` into the
    /// corresponding Booth digit.
    pub const fn from_window(window: [bool; 3]) -> Self {
        let [high, mid, low] = window;
        match (high, mid, low) {
            (false, false, false) | (true, true, true) => Self::Zero,
            (false, false, true) | (false, true, false) => Self::PlusOne,
            (false, true, true) => Self::PlusTwo,
            (true, false, false) => Self::MinusTwo,
            (true, false, true) | (true, true, false) => Self::MinusOne,
        }
    }
}

impl core::fmt::Display for BoothDigit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Zero => write!(f, "0"),
            Self::PlusOne => write!(f, "+1"),
            Self::PlusTwo => write!(f, "+2"),
            Self::MinusTwo => write!(f, "-2"),
            Self::MinusOne => write!(f, "-1"),
        }
    }
}

/// The number of Booth digits required to represent an `N`-bit
/// unsigned multiplier.
///
/// The formula is `ceil((N+1)/2)`, matching the Supranational
/// `rombooth` instantiation count (9 rows for `N = 17`).
#[must_use]
pub const fn digit_count(n: usize) -> usize {
    (n / 2) + 1
}

/// Extract the 3-bit Booth window for digit `i` of an `N`-bit
/// multiplier, returning `[high, mid, low]`.
///
/// Bit positions outside `[0, N)` are treated as zero.  The "low"
/// bit for digit 0 is the implicit position `-1` (always zero).
#[must_use]
pub fn window<const N: usize>(b: Bits<N>, digit_index: usize) -> [bool; 3]
where
    W<N>: BitWidth,
{
    let mid_pos = 2 * digit_index;
    let high = bit_at(b, mid_pos + 1);
    let mid = bit_at(b, mid_pos);
    let low = match digit_index {
        0 => false,
        _ => bit_at(b, mid_pos - 1),
    };
    [high, mid, low]
}

/// Encode the entire `N`-bit multiplier `b` as a sequence of Booth
/// digits, one per window.
///
/// # Examples
///
/// ```
/// use mul_cat::booth::digit::{encode_all, BoothDigit};
/// use rhdl_bits::bits;
///
/// let digits = encode_all(bits::<4>(0b1010));
/// assert_eq!(digits, vec![BoothDigit::MinusTwo, BoothDigit::MinusOne, BoothDigit::PlusOne]);
/// ```
#[must_use]
pub fn encode_all<const N: usize>(b: Bits<N>) -> Vec<BoothDigit>
where
    W<N>: BitWidth,
{
    (0..digit_count(N))
        .map(|i| BoothDigit::from_window(window(b, i)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhdl_bits::bits;

    #[test]
    fn digit_count_matches_supranational_17_bit() {
        assert_eq!(digit_count(17), 9);
    }

    #[test]
    fn digit_count_small_widths() {
        assert_eq!(digit_count(1), 1);
        assert_eq!(digit_count(2), 2);
        assert_eq!(digit_count(4), 3);
        assert_eq!(digit_count(8), 5);
        assert_eq!(digit_count(16), 9);
    }

    #[test]
    fn window_zero_uses_implicit_zero_below() {
        let b = bits::<4>(0b1011);
        let w = window(b, 0);
        // high = bit 1 = 1, mid = bit 0 = 1, low = 0 (implicit, position -1)
        assert_eq!(w, [true, true, false]);
    }

    #[test]
    fn window_beyond_msb_reads_zeros() {
        let b = bits::<4>(0b1000);
        let w = window(b, 2);
        assert_eq!(w, [false, false, true]);
    }

    #[test]
    fn from_window_covers_all_eight_patterns() {
        let patterns = [
            ([false, false, false], BoothDigit::Zero),
            ([false, false, true], BoothDigit::PlusOne),
            ([false, true, false], BoothDigit::PlusOne),
            ([false, true, true], BoothDigit::PlusTwo),
            ([true, false, false], BoothDigit::MinusTwo),
            ([true, false, true], BoothDigit::MinusOne),
            ([true, true, false], BoothDigit::MinusOne),
            ([true, true, true], BoothDigit::Zero),
        ];
        patterns.iter().for_each(|(w, d)| {
            assert_eq!(BoothDigit::from_window(*w), *d);
        });
    }

    #[test]
    fn encode_all_produces_expected_count() {
        let b = bits::<17>(12345);
        assert_eq!(encode_all(b).len(), 9);
    }
}
