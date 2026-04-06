//! Circuit-level Booth encoding and partial product generation.
//!
//! Each digit of the multiplier `B` is recoded into control signals
//! (negate, double, zero) that select among `{0, +A, +2A, -A, -2A}`
//! via a Mux cascade.  The shifted partial products are emitted as
//! `Bits(2N)` wires ready for carry-save tree reduction.

use crate::booth::digit::digit_count;
use crate::circuit::builder_ext::{
    emit_bin_bit, emit_concat, emit_const, emit_const_bit, emit_mux,
    emit_not_bit, emit_slice, emit_bin, width_to_u32,
};
use crate::error::Error;
use hdl_cat_ir::{BinOp, HdlGraphBuilder, WireId};

/// Control signals for one Booth digit (three single-bit wires).
struct BoothControls {
    negate: WireId,
    double: WireId,
    zero: WireId,
}

/// Extract bit `position` from `source` as a `Bit` wire.
///
/// Returns `zero_bit` when `position >= source_width`.
fn extract_bit(
    bld: HdlGraphBuilder,
    source: WireId,
    position: u32,
    source_width: u32,
    zero_bit: WireId,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    if position >= source_width {
        Ok((bld, zero_bit))
    } else {
        emit_slice(bld, source, position, position + 1)
    }
}

/// Compute Booth control signals for digit `digit_index` of the
/// `n`-bit multiplier on `b_wire`.
fn booth_controls(
    bld: HdlGraphBuilder,
    b_wire: WireId,
    digit_index: usize,
    n: u32,
    zero_bit: WireId,
) -> Result<(HdlGraphBuilder, BoothControls), Error> {
    let mid_pos = width_to_u32(2 * digit_index)?;

    // Window bits: [b2 (high), b1 (mid), b0 (low)]
    let (bld, b0) = match digit_index {
        0 => Ok((bld, zero_bit)),
        _ => extract_bit(bld, b_wire, mid_pos - 1, n, zero_bit),
    }?;
    let (bld, b1) = extract_bit(bld, b_wire, mid_pos, n, zero_bit)?;
    let (bld, b2) = extract_bit(bld, b_wire, mid_pos + 1, n, zero_bit)?;

    // Intermediate signals
    let (bld, b2_xor_b1) = emit_bin_bit(bld, BinOp::Xor, b2, b1)?;
    let (bld, b1_xor_b0) = emit_bin_bit(bld, BinOp::Xor, b1, b0)?;
    let (bld, not_b2_xor_b1) = emit_not_bit(bld, b2_xor_b1)?;
    let (bld, not_b1_xor_b0) = emit_not_bit(bld, b1_xor_b0)?;

    // zero  = (b2 XNOR b1) AND (b1 XNOR b0)
    let (bld, zero) = emit_bin_bit(bld, BinOp::And, not_b2_xor_b1, not_b1_xor_b0)?;
    // double = (b1 XNOR b0) AND (b2 XOR b1)
    let (bld, double) = emit_bin_bit(bld, BinOp::And, not_b1_xor_b0, b2_xor_b1)?;

    Ok((bld, BoothControls { negate: b2, double, zero }))
}

/// Build one shifted partial product wire of width `w = 2 * n`.
fn partial_product_wire(
    bld: HdlGraphBuilder,
    controls: &BoothControls,
    a_ext: WireId,
    a_doubled_ext: WireId,
    const_zero_w: WireId,
    digit_index: usize,
    w: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    // Select A or 2A based on double
    let (bld, selected) = emit_mux(bld, controls.double, a_ext, a_doubled_ext, w)?;

    // Conditional negation: 0 - selected = two's complement
    let (bld, neg_selected) = emit_bin(bld, BinOp::Sub, const_zero_w, selected, w)?;
    let (bld, signed_pp) = emit_mux(bld, controls.negate, selected, neg_selected, w)?;

    // Zero masking
    let (bld, pp) = emit_mux(bld, controls.zero, signed_pp, const_zero_w, w)?;

    // Shift by 2 * digit_index
    if digit_index == 0 {
        Ok((bld, pp))
    } else {
        let shift = width_to_u32(2 * digit_index)?;
        let (bld, low_zeros) = emit_const(bld, 0, shift)?;
        let (bld, pp_trunc) = emit_slice(bld, pp, 0, w - shift)?;
        emit_concat(bld, low_zeros, shift, pp_trunc, w - shift)
    }
}

/// Build all shifted partial product wires for an `n`-bit Booth
/// multiplier.
///
/// Returns the builder and a `Vec` of `Bits(2n)` wire IDs, one per
/// Booth digit.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if any IR instruction is rejected, or
/// [`Error::ZeroBitWidth`] if `n == 0`.
pub fn all_partial_product_wires(
    bld: HdlGraphBuilder,
    a_wire: WireId,
    b_wire: WireId,
    n: u32,
    operand_width: usize,
) -> Result<(HdlGraphBuilder, Vec<WireId>), Error> {
    let w = 2 * n; // product width
    let dc = digit_count(operand_width);

    // A zero-extended to 2N bits
    let (bld, const_zero_n) = emit_const(bld, 0, n)?;
    let (bld, a_ext) = emit_concat(bld, a_wire, n, const_zero_n, n)?;

    // A << 1 zero-extended to 2N bits
    let (bld, pad_low) = emit_const(bld, 0, 1)?;
    let (bld, a_shifted_n1) = emit_concat(bld, pad_low, 1, a_wire, n)?;
    let (bld, a_doubled_ext) = match n {
        0 => Err(Error::ZeroBitWidth),
        1 => Ok((bld, a_shifted_n1)),
        _ => {
            let (bld, const_zero_nm1) = emit_const(bld, 0, n - 1)?;
            emit_concat(bld, a_shifted_n1, n + 1, const_zero_nm1, n - 1)
        }
    }?;

    // Shared constants
    let (bld, const_zero_w) = emit_const(bld, 0, w)?;
    let (bld, zero_bit) = emit_const_bit(bld, false)?;

    // Build one partial product per digit
    (0..dc).try_fold(
        (bld, Vec::new()),
        |(bld, pp_wires), i| {
            let (bld, controls) = booth_controls(bld, b_wire, i, n, zero_bit)?;
            let (bld, pp_wire) = partial_product_wire(
                bld, &controls, a_ext, a_doubled_ext, const_zero_w, i, w,
            )?;
            Ok((
                bld,
                pp_wires.into_iter().chain(core::iter::once(pp_wire)).collect(),
            ))
        },
    )
}
