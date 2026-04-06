# mul-cat

A radix-4 Booth multiplier and schoolbook polynomial multiplier built on
[`comp-cat-rs`](https://github.com/MavenRain/comp-cat-rs) and the
[`hdl-cat`](https://github.com/MavenRain/hdl-cat) workspace.  The
carry-save reduction tree is modelled as a free category over a
linear-chain graph; each topology (Wallace, linear, ...) is a
`GraphMorphism` that the free category's universal property lifts to a
composed `ReductionDescriptor`.

This mirrors the architecture of the Supranational hardware multiplier
RTL ([supranational/hardware/rtl/multiplier](https://github.com/supranational/hardware/tree/master/rtl/multiplier)):
Booth recoding produces nine (for `N = 17`) sign-extended partial
products, a carry-save tree of 3-to-2 compressors reduces them to one
carry-save pair, and a single ripple addition yields the `2N`-bit product.

## Two Layers

| Layer | Module | What it produces |
|-------|--------|------------------|
| **Software evaluation** | `evaluate` | `MulResult<N>` via `u128` arithmetic |
| **Circuit** | `circuit` | `CircuitArrow`, simulation results, SystemVerilog |

Both layers share the same `Topology` trait and categorical reduction
framework.

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

Add the dependency:

```toml
[dependencies]
mul-cat = { git = "https://github.com/MavenRain/mul-cat" }
hdl-cat-bits = "0.1"
```

### Software evaluation

```rust
use mul_cat::evaluate::mul::booth_multiply;
use mul_cat::topology::wallace::Wallace;
use hdl_cat_bits::Bits;

let product = booth_multiply::<17>(Bits::<17>::new_wrapping(12345), Bits::<17>::new_wrapping(6789), &Wallace)
    .map(|r| r.to_wide_value())
    .ok();
assert_eq!(product, Some(12345_u128 * 6789));
```

### Circuit simulation

```rust
use mul_cat::circuit::mul::simulate_multiply;
use mul_cat::topology::wallace::Wallace;
use hdl_cat_bits::Bits;

let result = simulate_multiply::<8>(
    Bits::<8>::new_wrapping(12),
    Bits::<8>::new_wrapping(13),
    &Wallace,
);
assert_eq!(result.map(|m| m.to_wide_value()).ok(), Some(156));
```

### Circuit arrow construction

```rust
use mul_cat::circuit::mul::booth_multiplier_arrow;
use mul_cat::topology::wallace::Wallace;

let arrow = booth_multiplier_arrow::<8>(&Wallace);
assert!(arrow.is_ok());
```

### Verilog emission

```rust
use mul_cat::circuit::mul::booth_multiplier_module;
use mul_cat::topology::wallace::Wallace;

let module = booth_multiplier_module::<8>(&Wallace, "booth_mul_8");
assert!(module.is_ok());
```

### Schoolbook polynomial multiplication

```rust
use mul_cat::schoolbook::schoolbook_mul::schoolbook_multiply;
use mul_cat::topology::wallace::Wallace;
use hdl_cat_bits::Bits;

let a = [Bits::<8>::new_wrapping(3), Bits::<8>::new_wrapping(1), Bits::<8>::new_wrapping(4)];
let b = [Bits::<8>::new_wrapping(2), Bits::<8>::new_wrapping(7), Bits::<8>::new_wrapping(1)];
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
cargo test                                  # 83 unit tests + 13 doctests
RUSTFLAGS="-D warnings" cargo clippy --all-targets
cargo doc --no-deps --open
```

Property-based tests (via `proptest`) verify that both topologies agree
with native `u128` multiplication over the full 17-bit input range.
Circuit-level tests cross-validate against the software evaluation
layer, confirming bit-exact agreement across all 8-bit input pairs.

## License

Dual-licensed under either of

- MIT License
- Apache License, Version 2.0

at your option.
