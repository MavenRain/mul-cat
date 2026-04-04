//! Top-level Booth multiplier.
//!
//! Composes the three stages: Booth encoding, carry-save tree
//! reduction, and resolution into the `2N`-bit product.

use crate::bits_ext::{from_u128, mask};
use crate::error::Error;
use crate::evaluate::booth_stage::booth_partial_products;
use crate::evaluate::tree_stage::reduce_terms;
use crate::topology::Topology;
use rhdl_bits::{BitWidth, Bits, W};

/// The `2N`-bit product of two `N`-bit operands, stored as low and
/// high `N`-bit halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct MulResult<const N: usize>
where
    W<N>: BitWidth,
{
    low: Bits<N>,
    high: Bits<N>,
}

impl<const N: usize> MulResult<N>
where
    W<N>: BitWidth,
{
    /// Construct a result from its two halves.
    pub const fn new(low: Bits<N>, high: Bits<N>) -> Self {
        Self { low, high }
    }

    /// The low `N` bits of the product.
    #[must_use]
    pub const fn low(self) -> Bits<N> {
        self.low
    }

    /// The high `N` bits of the product.
    #[must_use]
    pub const fn high(self) -> Bits<N> {
        self.high
    }

    /// Reassemble the full `2N`-bit product as a `u128`.
    #[must_use]
    pub const fn to_wide_value(self) -> u128 {
        self.low.raw() | (self.high.raw() << N)
    }
}

/// Maximum supported operand bit width.
///
/// The internal computation uses `u128` for the `2N`-bit product,
/// so operands are limited to `64` bits.
pub const MAX_OPERAND_WIDTH: usize = 64;

/// Multiply two `N`-bit operands via radix-4 Booth encoding and the
/// specified carry-save reduction topology.
///
/// # Errors
///
/// - [`Error::ZeroBitWidth`] if `N == 0`.
/// - [`Error::BitWidthTooLarge`] if `N > 64`.
/// - Any error produced by the topology's reduction schedule.
///
/// # Examples
///
/// ```
/// use mul_cat::evaluate::mul::booth_multiply;
/// use mul_cat::topology::wallace::Wallace;
/// use rhdl_bits::bits;
///
/// let product = booth_multiply::<17>(bits::<17>(12345), bits::<17>(6789), &Wallace)
///     .map(|r| r.to_wide_value())
///     .ok();
/// assert_eq!(product, Some(12345_u128 * 6789));
/// ```
pub fn booth_multiply<const N: usize>(
    a: Bits<N>,
    b: Bits<N>,
    topology: &impl Topology,
) -> Result<MulResult<N>, Error>
where
    W<N>: BitWidth,
{
    match N {
        0 => Err(Error::ZeroBitWidth),
        width if width > MAX_OPERAND_WIDTH => Err(Error::BitWidthTooLarge {
            width,
            max: MAX_OPERAND_WIDTH,
        }),
        _ => {
            let partials = booth_partial_products(a, b);
            let pair = reduce_terms(topology, &partials, N)?;
            let product = pair.resolve(mask(2 * N));
            let lo_mask = mask(N);
            let low = from_u128::<N>(product & lo_mask);
            let high = from_u128::<N>((product >> N) & lo_mask);
            Ok(MulResult::new(low, high))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::linear::Linear;
    use crate::topology::wallace::Wallace;
    use proptest::prelude::*;
    use rhdl_bits::bits;

    #[test]
    fn zero_times_zero_is_zero() -> Result<(), Error> {
        let r = booth_multiply::<17>(bits::<17>(0), bits::<17>(0), &Wallace)?;
        assert_eq!(r.to_wide_value(), 0);
        Ok(())
    }

    #[test]
    fn identity_multiplications() -> Result<(), Error> {
        let r = booth_multiply::<17>(bits::<17>(1), bits::<17>(12345), &Wallace)?;
        assert_eq!(r.to_wide_value(), 12345);
        let r = booth_multiply::<17>(bits::<17>(12345), bits::<17>(1), &Wallace)?;
        assert_eq!(r.to_wide_value(), 12345);
        Ok(())
    }

    #[test]
    fn max_times_max_17_bit() -> Result<(), Error> {
        let m: u128 = (1 << 17) - 1;
        let r = booth_multiply::<17>(bits::<17>(m), bits::<17>(m), &Wallace)?;
        assert_eq!(r.to_wide_value(), m * m);
        Ok(())
    }

    #[test]
    fn wallace_and_linear_agree_on_random_inputs() -> Result<(), Error> {
        let cases: [(u128, u128); 6] = [
            (0, 0),
            (1, 1),
            (12345, 6789),
            (131_071, 131_071),
            (0x1_5555, 0x0_AAAA),
            (0x1FFFF, 2),
        ];
        cases.iter().try_for_each(|(a, b)| {
            let aa = bits::<17>(*a);
            let bb = bits::<17>(*b);
            let wallace = booth_multiply::<17>(aa, bb, &Wallace)?.to_wide_value();
            let linear = booth_multiply::<17>(aa, bb, &Linear)?.to_wide_value();
            assert_eq!(wallace, linear);
            assert_eq!(wallace, a * b);
            Ok(())
        })
    }

    #[test]
    fn eight_bit_multiplication() -> Result<(), Error> {
        let r = booth_multiply::<8>(bits::<8>(200), bits::<8>(199), &Wallace)?;
        assert_eq!(r.to_wide_value(), 200 * 199);
        Ok(())
    }

    #[test]
    fn thirty_two_bit_multiplication() -> Result<(), Error> {
        let a: u128 = 0xDEAD_BEEF;
        let b: u128 = 0xCAFE_BABE;
        let r = booth_multiply::<32>(bits::<32>(a), bits::<32>(b), &Wallace)?;
        assert_eq!(r.to_wide_value(), a * b);
        Ok(())
    }

    proptest! {
        #[test]
        fn wallace_matches_native_multiplication_17_bit(
            a in 0_u128..(1 << 17),
            b in 0_u128..(1 << 17),
        ) {
            let r = booth_multiply::<17>(bits::<17>(a), bits::<17>(b), &Wallace)
                .map(MulResult::to_wide_value)
                .ok();
            prop_assert_eq!(r, Some(a * b));
        }

        #[test]
        fn linear_matches_native_multiplication_17_bit(
            a in 0_u128..(1 << 17),
            b in 0_u128..(1 << 17),
        ) {
            let r = booth_multiply::<17>(bits::<17>(a), bits::<17>(b), &Linear)
                .map(MulResult::to_wide_value)
                .ok();
            prop_assert_eq!(r, Some(a * b));
        }
    }

    #[test]
    fn low_and_high_halves_reassemble() -> Result<(), Error> {
        let r = booth_multiply::<8>(bits::<8>(200), bits::<8>(199), &Wallace)?;
        let product = 200_u128 * 199;
        assert_eq!(r.low().raw(), product & 0xFF);
        assert_eq!(r.high().raw(), (product >> 8) & 0xFF);
        Ok(())
    }
}
