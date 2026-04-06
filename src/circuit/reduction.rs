//! Topology-driven carry-save tree reduction on IR wires.
//!
//! The circuit-level analogue of [`crate::evaluate::tree_stage`]:
//! the [`Topology`] trait schedules which groups of three wires to
//! compress at each level, and the builder threads CSA compressor
//! sub-graphs accordingly.

use crate::circuit::builder_ext::emit_const;
use crate::circuit::csa::csa_compress_three_wires;
use crate::error::Error;
use crate::interpret::descriptor::CsaGrouping;
use crate::topology::Topology;
use hdl_cat_ir::{HdlGraphBuilder, WireId};

/// Apply a single level's [`CsaGrouping`] to the current wire list.
fn apply_grouping(
    bld: HdlGraphBuilder,
    grouping: &CsaGrouping,
    wires: &[WireId],
    width: u32,
) -> Result<(HdlGraphBuilder, Vec<WireId>), Error> {
    // Compress each triple
    let (bld, compressed) = grouping.triples().iter().try_fold(
        (bld, Vec::new()),
        |(bld, acc), triple| {
            let (bld, carry, sum) = csa_compress_three_wires(
                bld, wires[triple[0]], wires[triple[1]], wires[triple[2]], width,
            )?;
            Ok::<_, Error>((
                bld,
                acc.into_iter().chain([carry, sum]).collect(),
            ))
        },
    )?;

    // Passthroughs reuse existing wire IDs
    let all: Vec<WireId> = compressed
        .into_iter()
        .chain(grouping.passthroughs().iter().map(|i| wires[*i]))
        .collect();

    Ok((bld, all))
}

/// Reduce a list of `Bits(width)` wires to a carry-sum pair via
/// the specified topology.
///
/// Returns `(builder, carry_wire, sum_wire)`.  For zero or one input
/// terms the missing wire is a constant zero.
///
/// # Errors
///
/// Returns topology or IR errors via [`Error`].
pub fn reduce_wires(
    bld: HdlGraphBuilder,
    topology: &impl Topology,
    wires: Vec<WireId>,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId, WireId), Error> {
    let initial_count = wires.len();
    match initial_count {
        0 => {
            let (bld, z1) = emit_const(bld, 0, width)?;
            let (bld, z2) = emit_const(bld, 0, width)?;
            Ok((bld, z1, z2))
        }
        1 => {
            let (bld, z) = emit_const(bld, 0, width)?;
            Ok((bld, z, wires[0]))
        }
        2 => Ok((bld, wires[0], wires[1])),
        _ => {
            let level_count = topology.level_count(initial_count);
            let (bld, final_wires) = (0..level_count).try_fold(
                (bld, wires),
                |(bld, current), level| {
                    let grouping = topology.level_grouping(initial_count, level)?;
                    apply_grouping(bld, &grouping, &current, width)
                },
            )?;
            match final_wires.as_slice() {
                [a, b] => Ok((bld, *a, *b)),
                _ => Err(Error::GroupingMismatch {
                    input_count: initial_count,
                    triples: 0,
                    passthroughs: final_wires.len(),
                }),
            }
        }
    }
}
