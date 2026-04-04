# mul-cat

A radix-4 Booth multiplier and schoolbook polynomial multiplier built on
[`comp-cat-rs`](https://github.com/MavenRain/comp-cat-rs) and
[`rhdl-bits`](https://github.com/samitbasu/rhdl).  The carry-save
reduction tree is modelled as a free category over a linear-chain graph;
each topology (Wallace, linear, ...) is a `GraphMorphism` that the free
category's universal property lifts to a composed `ReductionDescriptor`.

This mirrors the architecture of the Supranational hardware multiplier
RTL ([supranational/hardware/rtl/multiplier](https://github.com/supranational/hardware/tree/master/rtl/multiplier)):
Booth recoding produces nine (for `N = 17`) sign-extended partial
products, a carry-save tree of 3-to-2 compressors reduces them to one
carry-save pair, and a single ripple addition yields the `2N`-bit product.

## Pipeline

```text
Topology -> ReductionGraph + ReductionMorphism
         -> full_reduction_path (Path through all levels)
         -> interpret (compose ReductionDescriptor along the path)
         -> evaluate on Booth partial products
         -> CarrySavePair
         -> MulResult<N>
```

## Usage

Add the dependency (RHDL is pulled directly from GitHub):

```toml
[dependencies]
mul-cat = { git = "https://github.com/MavenRain/mul-cat" }
rhdl-bits = { git = "https://github.com/samitbasu/rhdl" }
```

Multiply two 17-bit operands via Booth encoding and a Wallace tree:

```rust
use mul_cat::evaluate::mul::booth_multiply;
use mul_cat::topology::wallace::Wallace;
use rhdl_bits::bits;

let product = booth_multiply::<17>(bits::<17>(12345), bits::<17>(6789), &Wallace)
    .map(|r| r.to_wide_value())
    .ok();
assert_eq!(product, Some(12345_u128 * 6789));
```

Compute a schoolbook polynomial product with per-column carry-save
reduction:

```rust
use mul_cat::schoolbook::schoolbook_mul::schoolbook_multiply;
use mul_cat::topology::wallace::Wallace;
use rhdl_bits::bits;

let a = [bits::<8>(3), bits::<8>(1), bits::<8>(4)];
let b = [bits::<8>(2), bits::<8>(7), bits::<8>(1)];
let result = schoolbook_multiply::<8>(&a, &b, 4, &Wallace).ok();
assert!(result.is_some());
```

## Topologies

| Topology | Depth (for `K` terms) | Parallelism |
|----------|-----------------------|-------------|
| [`Wallace`](src/topology/wallace.rs) | `O(log K)` | maximum triples per level |
| [`Linear`](src/topology/linear.rs)   | `K - 2`    | one triple per level (reference model) |

New topologies slot in by implementing the `Topology` trait.

## Testing

```sh
cargo test                                  # 74 unit tests + 10 doctests
RUSTFLAGS="-D warnings" cargo clippy --all-targets
cargo doc --no-deps --open
```

Property-based tests (via `proptest`) verify that both topologies agree
with native `u128` multiplication over the full 17-bit input range, and
that the schoolbook column-recombination at the chosen base matches
`A(base) * B(base)` for randomly generated coefficient arrays.

## License

Dual-licensed under either of

- MIT License
- Apache License, Version 2.0

at your option.
