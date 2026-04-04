// `needless_for_each` conflicts with the project convention
// forbidding `for`/`loop`; iterator combinators are mandatory.
#![allow(clippy::needless_for_each)]
// `tuple_array_conversions` prefers `.into()`; explicit tuple
// patterns make the Booth window decode table clearer.
#![allow(clippy::tuple_array_conversions)]

//! Multiplier built on comp-cat-rs and RHDL.
//!
//! This crate implements radix-4 Booth-encoded multipliers with
//! configurable carry-save tree reductions (Wallace, linear, etc.)
//! using the free category framework from [`comp_cat_rs`].  A
//! carry-save tree is modeled as a linear-chain graph where each
//! edge represents one round of 3-to-2 compression, and the
//! [`interpret`](comp_cat_rs::collapse::free_category::interpret)
//! function composes reduction descriptors along the path.
//!
//! Hardware bit types are provided by [`rhdl_bits`] (`Bits<N>`).
//!
//! # Architecture
//!
//! ```text
//! Topology -> ReductionGraph + ReductionMorphism
//!          -> full_reduction_path (build Path through all levels)
//!          -> interpret (compose ReductionDescriptor along path)
//!          -> evaluate on Booth partial products
//!          -> CarrySavePair
//!          -> MulResult<N>
//! ```
//!
//! # Examples
//!
//! ```
//! use mul_cat::evaluate::mul::booth_multiply;
//! use mul_cat::topology::wallace::Wallace;
//! use rhdl_bits::bits;
//!
//! let product = booth_multiply::<17>(bits::<17>(12345), bits::<17>(6789), &Wallace)
//!     .map(|r| r.to_wide_value())
//!     .ok();
//! assert_eq!(product, Some(12345_u128 * 6789));
//! ```
//!
//! # Reference
//!
//! The structure mirrors the Supranational hardware multiplier RTL
//! (`supranational/hardware/rtl/multiplier`): radix-4 Booth recoding
//! produces sign-extended partial products, which are reduced by a
//! carry-save tree of 3-to-2 compressors (full adders) to a final
//! carry-save pair; one ripple add yields the product.

pub mod bits_ext;
pub mod booth;
pub mod carry_save;
pub mod error;
pub mod evaluate;
pub mod graph;
pub mod interpret;
pub mod schoolbook;
pub mod topology;
