//! Schoolbook polynomial multiplier with per-column carry-save
//! reduction.
//!
//! Given two coefficient arrays `a, b` of length `K`, each
//! coefficient `N` bits, the schoolbook product consists of `K * K`
//! elementary products `a[i] * b[j]` placed into a grid and summed
//! column by column.  The low `WORD_LEN` bits of `a[i] * b[j]` drop
//! into column `i + j` and the high `2N - WORD_LEN` bits drop into
//! column `i + j + 1`.  Each column is then reduced independently
//! to a carry-save pair using the same categorical tree topology
//! as the coefficient multiplier.  This mirrors the Supranational
//! `schoolbook_mul` RTL.

pub mod grid;
pub mod schoolbook_mul;
