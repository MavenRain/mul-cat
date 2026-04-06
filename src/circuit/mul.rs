//! Top-level circuit multiplier: arrow construction, simulation,
//! and Verilog module emission.

use crate::circuit::booth::all_partial_product_wires;
use crate::circuit::builder_ext::{emit_bin, emit_slice, width_to_u32};
use crate::circuit::reduction::reduce_wires;
use crate::error::Error;
use crate::evaluate::mul::MAX_OPERAND_WIDTH;
use crate::evaluate::mul::MulResult;
use crate::topology::Topology;
use hdl_cat_bits::Bits;
use hdl_cat_circuit::{CircuitArrow, CircuitTensor, Obj};
use hdl_cat_ir::{BinOp, HdlGraphBuilder, WireTy};
use hdl_cat_kind::Hw;
use hdl_cat_sim::Testbench;
use hdl_cat_sync::Sync;

/// The arrow type for the Booth multiplier:
/// `(Bits<N> ⊗ Bits<N>) -> (Bits<N> ⊗ Bits<N>)`.
///
/// Input tensor: `(multiplicand, multiplier)`.
/// Output tensor: `(low_half, high_half)`.
pub type BoothMulArrow<const N: usize> = CircuitArrow<
    CircuitTensor<Obj<Bits<N>>, Obj<Bits<N>>>,
    CircuitTensor<Obj<Bits<N>>, Obj<Bits<N>>>,
>;

/// Validate the operand width for arrow construction.
const fn validate_arrow_params(n: usize) -> Result<(), Error> {
    match n {
        0 => Err(Error::ZeroBitWidth),
        width if width > MAX_OPERAND_WIDTH => Err(Error::BitWidthTooLarge {
            width,
            max: MAX_OPERAND_WIDTH,
        }),
        _ => Ok(()),
    }
}

/// Build a purely combinational Booth multiplier as a
/// [`CircuitArrow`].
///
/// The arrow accepts `(Bits<N>, Bits<N>)` and produces
/// `(low_N_bits, high_N_bits)` of the `2N`-bit product.  The
/// carry-save tree is scheduled by the given [`Topology`].
///
/// # Errors
///
/// - [`Error::ZeroBitWidth`] if `N == 0`.
/// - [`Error::BitWidthTooLarge`] if `N > 64`.
/// - Any IR builder or topology error.
///
/// # Examples
///
/// ```
/// use mul_cat::circuit::mul::booth_multiplier_arrow;
/// use mul_cat::topology::wallace::Wallace;
///
/// let arrow = booth_multiplier_arrow::<8>(&Wallace);
/// assert!(arrow.is_ok());
/// ```
pub fn booth_multiplier_arrow<const N: usize>(
    topology: &impl Topology,
) -> Result<BoothMulArrow<N>, Error> {
    validate_arrow_params(N)?;
    let n = width_to_u32(N)?;
    let w = 2 * n;

    // Input wires
    let (bld, a_wire) = HdlGraphBuilder::new().with_wire(WireTy::Bits(n));
    let (bld, b_wire) = bld.with_wire(WireTy::Bits(n));

    // Booth partial products
    let (bld, pp_wires) = all_partial_product_wires(bld, a_wire, b_wire, n, N)?;

    // CSA tree reduction
    let (bld, carry_wire, sum_wire) = reduce_wires(bld, topology, pp_wires, w)?;

    // Final ripple addition
    let (bld, product) = emit_bin(bld, BinOp::Add, carry_wire, sum_wire, w)?;

    // Split into N-bit halves
    let (bld, low) = emit_slice(bld, product, 0, n)?;
    let (bld, high) = emit_slice(bld, product, n, w)?;

    Ok(CircuitArrow::from_raw_parts(
        bld.build(),
        vec![a_wire, b_wire],
        vec![low, high],
    ))
}

/// Simulate a single multiplication through the circuit and return
/// the result.
///
/// Builds the multiplier arrow, wraps it in a [`Testbench`], feeds
/// the operands for one cycle, and extracts the product.
///
/// # Errors
///
/// Returns arrow-construction, simulation, or bit-conversion errors.
///
/// # Examples
///
/// ```
/// use mul_cat::circuit::mul::simulate_multiply;
/// use mul_cat::topology::wallace::Wallace;
/// use hdl_cat_bits::Bits;
///
/// let r = simulate_multiply::<8>(
///     Bits::<8>::new_wrapping(12),
///     Bits::<8>::new_wrapping(13),
///     &Wallace,
/// );
/// assert_eq!(r.map(|m| m.to_wide_value()).ok(), Some(156));
/// ```
pub fn simulate_multiply<const N: usize>(
    a: Bits<N>,
    b: Bits<N>,
    topology: &impl Topology,
) -> Result<MulResult<N>, Error> {
    let arrow = booth_multiplier_arrow::<N>(topology)?;
    let machine = Sync::lift_comb(arrow);
    let testbench = Testbench::new(machine);
    let input = a.to_bits_seq().concat(b.to_bits_seq());
    let samples = testbench.run(vec![input]).run()?;
    let output = samples
        .first()
        .map(hdl_cat_sim::TimedSample::value)
        .ok_or(Error::ZeroBitWidth)?;
    let (low_bits, high_bits) = output.clone().split_at(N);
    let low = Bits::<N>::from_bits_seq(&low_bits)?;
    let high = Bits::<N>::from_bits_seq(&high_bits)?;
    Ok(MulResult::new(low, high))
}

/// Emit a Verilog [`Module`](hdl_cat_verilog::Module) for the Booth
/// multiplier.
///
/// The module is purely combinational (no clock or reset).
///
/// # Errors
///
/// Returns arrow-construction or Verilog-emission errors.
///
/// # Examples
///
/// ```
/// use mul_cat::circuit::mul::booth_multiplier_module;
/// use mul_cat::topology::wallace::Wallace;
///
/// let module = booth_multiplier_module::<8>(&Wallace, "mul8");
/// assert!(module.is_ok());
/// ```
pub fn booth_multiplier_module<const N: usize>(
    topology: &impl Topology,
    module_name: &str,
) -> Result<hdl_cat_verilog::Module, Error> {
    let arrow = booth_multiplier_arrow::<N>(topology)?;
    hdl_cat_verilog::emit_graph(
        arrow.graph(),
        module_name,
        arrow.inputs(),
        arrow.outputs(),
    )
    .run()
    .map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate::mul::booth_multiply;
    use crate::topology::linear::Linear;
    use crate::topology::wallace::Wallace;
    use proptest::prelude::*;

    #[test]
    fn arrow_builds_for_eight_bit() -> Result<(), Error> {
        let arrow = booth_multiplier_arrow::<8>(&Wallace)?;
        assert_eq!(arrow.inputs().len(), 2);
        assert_eq!(arrow.outputs().len(), 2);
        Ok(())
    }

    #[test]
    fn arrow_builds_for_seventeen_bit() -> Result<(), Error> {
        let arrow = booth_multiplier_arrow::<17>(&Wallace)?;
        assert_eq!(arrow.inputs().len(), 2);
        assert_eq!(arrow.outputs().len(), 2);
        Ok(())
    }

    #[test]
    fn arrow_rejects_zero_width() {
        assert!(booth_multiplier_arrow::<0>(&Wallace).is_err());
    }

    #[test]
    fn simulate_small_product() -> Result<(), Error> {
        let r = simulate_multiply::<8>(
            Bits::<8>::new_wrapping(12),
            Bits::<8>::new_wrapping(13),
            &Wallace,
        )?;
        assert_eq!(r.to_wide_value(), 156);
        Ok(())
    }

    #[test]
    fn simulate_zero_times_anything() -> Result<(), Error> {
        let r = simulate_multiply::<8>(
            Bits::<8>::new_wrapping(0),
            Bits::<8>::new_wrapping(255),
            &Wallace,
        )?;
        assert_eq!(r.to_wide_value(), 0);
        Ok(())
    }

    #[test]
    fn simulate_matches_eval_17_bit() -> Result<(), Error> {
        let cases: [(u128, u128); 5] = [
            (0, 0),
            (1, 1),
            (12345, 6789),
            (131_071, 131_071),
            (0x1FFFF, 2),
        ];
        cases.iter().try_for_each(|(a, b)| {
            let aa = Bits::<17>::new_wrapping(*a);
            let bb = Bits::<17>::new_wrapping(*b);
            let eval_result = booth_multiply::<17>(aa, bb, &Wallace)?.to_wide_value();
            let sim_result = simulate_multiply::<17>(aa, bb, &Wallace)?.to_wide_value();
            assert_eq!(sim_result, eval_result);
            assert_eq!(sim_result, a * b);
            Ok(())
        })
    }

    #[test]
    fn wallace_and_linear_circuits_agree() -> Result<(), Error> {
        let a = Bits::<8>::new_wrapping(200);
        let b = Bits::<8>::new_wrapping(199);
        let w = simulate_multiply::<8>(a, b, &Wallace)?.to_wide_value();
        let l = simulate_multiply::<8>(a, b, &Linear)?.to_wide_value();
        assert_eq!(w, l);
        assert_eq!(w, 200 * 199);
        Ok(())
    }

    #[test]
    fn verilog_module_emits_for_eight_bit() -> Result<(), Error> {
        let _module = booth_multiplier_module::<8>(&Wallace, "booth_mul_8")?;
        Ok(())
    }

    proptest! {
        #[test]
        fn circuit_matches_eval_8_bit(
            a in 0_u128..256,
            b in 0_u128..256,
        ) {
            let aa = Bits::<8>::new_wrapping(a);
            let bb = Bits::<8>::new_wrapping(b);
            let eval = booth_multiply::<8>(aa, bb, &Wallace)
                .map(MulResult::to_wide_value)
                .ok();
            let sim = simulate_multiply::<8>(aa, bb, &Wallace)
                .map(MulResult::to_wide_value)
                .ok();
            prop_assert_eq!(sim, eval);
            prop_assert_eq!(sim, Some(a * b));
        }
    }
}
