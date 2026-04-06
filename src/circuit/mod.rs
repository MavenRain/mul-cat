//! Circuit-level Booth multiplier: [`hdl_cat_circuit::CircuitArrow`]
//! construction, simulation, and Verilog emission.
//!
//! This module mirrors [`crate::evaluate`] but operates on IR
//! instruction graphs rather than `u128` values.  The same
//! [`Topology`](crate::topology::Topology) trait drives the
//! carry-save tree schedule at both levels.

pub mod booth;
pub mod builder_ext;
pub mod csa;
pub mod mul;
pub mod reduction;
