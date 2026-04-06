//! Builder extension helpers for constructing [`hdl_cat_ir::HdlGraph`]
//! graphs in a purely functional style.
//!
//! Each helper consumes the [`HdlGraphBuilder`], appends a wire and
//! an instruction, and returns the updated builder together with
//! the new output [`WireId`].

use crate::error::Error;
use hdl_cat_ir::{BinOp, HdlGraphBuilder, Op, WireId, WireTy};
use hdl_cat_kind::BitSeq;

/// Convert a `u128` value to a [`BitSeq`] of the given width (LSB first).
pub fn u128_to_bitseq(value: u128, width: u32) -> BitSeq {
    (0..width).map(|i| (value >> i) & 1 != 0).collect()
}

/// Convert a `usize` to `u32`, failing if it overflows.
///
/// # Errors
///
/// Returns [`Error::BitWidthTooLarge`] when the value exceeds `u32::MAX`.
pub fn width_to_u32(n: usize) -> Result<u32, Error> {
    u32::try_from(n).map_err(|_| Error::BitWidthTooLarge {
        width: n,
        max: MAX_OPERAND_WIDTH,
    })
}

/// Maximum operand width re-exported for error messages.
const MAX_OPERAND_WIDTH: usize = crate::evaluate::mul::MAX_OPERAND_WIDTH;

/// Emit a constant `Bits(width)` wire with the given value.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_const(
    bld: HdlGraphBuilder,
    value: u128,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let ty = WireTy::Bits(width);
    let (bld, out) = bld.with_wire(ty.clone());
    bld.with_instruction(Op::Const { bits: u128_to_bitseq(value, width), ty }, vec![], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit a constant single-bit wire.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_const_bit(
    bld: HdlGraphBuilder,
    value: bool,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let ty = WireTy::Bit;
    let bits: BitSeq = core::iter::once(value).collect();
    let (bld, out) = bld.with_wire(ty.clone());
    bld.with_instruction(Op::Const { bits, ty }, vec![], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit a binary operation on two `Bits(width)` wires.
///
/// For comparison operators the output is `Bit`; otherwise `Bits(width)`.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_bin(
    bld: HdlGraphBuilder,
    op: BinOp,
    a: WireId,
    b: WireId,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let out_ty = if op.is_comparison() {
        WireTy::Bit
    } else {
        WireTy::Bits(width)
    };
    let (bld, out) = bld.with_wire(out_ty);
    bld.with_instruction(Op::Bin(op), vec![a, b], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit a binary operation on two `Bit` wires.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_bin_bit(
    bld: HdlGraphBuilder,
    op: BinOp,
    a: WireId,
    b: WireId,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (bld, out) = bld.with_wire(WireTy::Bit);
    bld.with_instruction(Op::Bin(op), vec![a, b], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit NOT on a `Bit` wire.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_not_bit(
    bld: HdlGraphBuilder,
    a: WireId,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (bld, out) = bld.with_wire(WireTy::Bit);
    bld.with_instruction(Op::Not, vec![a], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit a 2:1 multiplexer on `Bits(width)` data wires.
///
/// When `sel` is 0 the output is `false_arm`; when 1 it is `true_arm`.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_mux(
    bld: HdlGraphBuilder,
    sel: WireId,
    false_arm: WireId,
    true_arm: WireId,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (bld, out) = bld.with_wire(WireTy::Bits(width));
    bld.with_instruction(Op::Mux, vec![sel, false_arm, true_arm], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit a bit-range extraction: bits `[lo, hi)` of `src`.
///
/// The output type is `Bit` when `hi - lo == 1`, otherwise
/// `Bits(hi - lo)`.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_slice(
    bld: HdlGraphBuilder,
    src: WireId,
    lo: u32,
    hi: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let out_ty = match hi - lo {
        1 => WireTy::Bit,
        w => WireTy::Bits(w),
    };
    let (bld, out) = bld.with_wire(out_ty);
    bld.with_instruction(Op::Slice { lo, hi }, vec![src], out)
        .map(|b| (b, out))
        .map_err(Error::from)
}

/// Emit a bus concatenation: `result = {high, low}`.
///
/// The low operand occupies the least-significant bits of the result.
///
/// # Errors
///
/// Returns [`Error::HdlCat`] if the IR builder rejects the instruction.
pub fn emit_concat(
    bld: HdlGraphBuilder,
    low: WireId,
    low_width: u32,
    high: WireId,
    high_width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (bld, out) = bld.with_wire(WireTy::Bits(low_width + high_width));
    bld.with_instruction(
        Op::Concat { low_width, high_width },
        vec![low, high],
        out,
    )
        .map(|b| (b, out))
        .map_err(Error::from)
}
