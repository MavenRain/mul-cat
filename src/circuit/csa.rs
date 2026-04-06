//! Carry-save 3-to-2 compressor as an IR sub-graph.
//!
//! The circuit-level analogue of [`crate::carry_save::compress_three`]:
//! three `Bits(width)` wires are reduced to a carry-sum pair via
//! bitwise majority (carry) and XOR (sum), with the carry
//! left-shifted by one.

use crate::circuit::builder_ext::{emit_bin, emit_concat, emit_const, emit_slice};
use crate::error::Error;
use hdl_cat_ir::{BinOp, HdlGraphBuilder, WireId};

/// Compress three `Bits(width)` wires into a carry-sum pair.
///
/// Returns `(builder, carry_wire, sum_wire)` where both outputs are
/// `Bits(width)` and `carry + sum` equals `a + b + c` modulo
/// `2^width`.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if any IR instruction is rejected.
pub fn csa_compress_three_wires(
    bld: HdlGraphBuilder,
    a: WireId,
    b: WireId,
    c: WireId,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId, WireId), Error> {
    // sum = a ^ b ^ c
    let (bld, ab_xor) = emit_bin(bld, BinOp::Xor, a, b, width)?;
    let (bld, sum) = emit_bin(bld, BinOp::Xor, ab_xor, c, width)?;

    // raw_carry = (a & b) | (a & c) | (b & c)
    let (bld, first_and) = emit_bin(bld, BinOp::And, a, b, width)?;
    let (bld, second_and) = emit_bin(bld, BinOp::And, a, c, width)?;
    let (bld, third_and) = emit_bin(bld, BinOp::And, b, c, width)?;
    let (bld, majority_partial) = emit_bin(bld, BinOp::Or, first_and, second_and, width)?;
    let (bld, raw_carry) = emit_bin(bld, BinOp::Or, majority_partial, third_and, width)?;

    // carry = raw_carry << 1, masked to width
    let (bld, zero_1) = emit_const(bld, 0, 1)?;
    let (bld, carry_top) = emit_slice(bld, raw_carry, 0, width - 1)?;
    let (bld, carry) = emit_concat(bld, zero_1, 1, carry_top, width - 1)?;

    Ok((bld, carry, sum))
}
