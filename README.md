# SmartPacker

![logo](./logo.png)
![Pages](https://img.shields.io/badge/r0.2.0-beta-brightgreen.svg?style=flat-square)

A 3D packing algorithm library for load optimization of logistics cabinets and containers.

## Application

<img alt="BestLoad" src="https://github.com/user-attachments/assets/238d8f44-4011-4f69-85ec-ba6e8c44973b" width="100" />  
[优载 BestLoad](https://github.com/niyongsheng/best-load)

## Features

- **Vertical bottom support** — placement is accepted only when the support's top face exactly holds the item's bottom face (`y1 == y0`), not on arbitrary projection overlap.
- **Per-item "allowed float ratio"** — each item may declare `Item::allowed_float_ratio` (0..=1, 0.25 recommended by default):
  the bottom face is legal when the support ratio ≥ `1 − allowed_float_ratio`, with all four bottom corners landed as the last resort; `0` requires full support, `1` allows unlimited overhang.
- **Gravity correction and snapping** — `fix_point` gap-snapping on all three axes (to floor/walls); `bigger_first`, `distribute_items` (multi-bin distribution), `binding` (binding groups).
- **Numerical stability** — `f64` with ROUND_HALF_EVEN rounding, EPS (1e-9) tolerance; zero external runtime dependencies in the core library (only `std`),
  with optional `serde` / `plot` (PNG rendering) features.
- **Quality-gate tests** — `tests/no_floating.rs` asserts the support rules on packing output; `cargo run --example floating_check` for manual batch scanning.

## Quick Start

```bash
cargo add smartpacker
```

```rust
use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

fn main() {
    let mut p = Packer::new();
    p.add_bin(Bin::new("example", [30.0, 10.0, 15.0], 99.0));
    p.add_item(Item::new("test", "test", ItemType::Cube, [9.0, 8.0, 7.0], 1.0, 1, 100, true, "red", 0.25));
    p.pack(&PackOptions::default());

    for it in &p.bins[0].items {
        println!("{} @ {:?} rot={}", it.partno, it.position, it.rotation_type);
    }
}
```

More runnable examples:

```bash
cargo run --example readme_simple   # 30×10×15 bin + 5 items
cargo run --example cylinder_mixed  # cylinder + cube mixed
cargo run --example multi_bin       # two bins + distribute_items
cargo run --example stability       # two bottom-support rules (float ratio / four-corner support)
cargo run --example binding         # binding groups
cargo run --example plot --features plot        # render packing result to PNG
```

## Technical Documentation

The algorithm internals (sorting chain, `put_item` heuristics, gravity correction, bottom support, binding groups, center-of-mass distribution),
data model, numerical semantics and test strategy are fully documented in **[`smartpacker/doc.md`](./smartpacker/doc.md)**.

## smartpacker-server

HTTP packing service (the server-side contract companion for the best-load app), listening on `0.0.0.0:5050` by default:

```bash
SMARTPACKER_ADDR=127.0.0.1:5050 cargo run -p smartpacker-server
```

| Route | Method | Behavior |
|---|---|---|
| `/` | GET | Service banner |
| `/getAllData` | POST | Returns the built-in sample data + `Success: true` |
| `/calPacking` | POST | Input `{box, item, binding}`, returns `data.{box, fitItem, unfitItem}` |

Input conventions are documented in the module docs of [`smartpacker-server/src/lib.rs`](./smartpacker-server/src/lib.rs):
`box[0].openTop[0]` is used as `put_type`, items are expanded by `count`, `type==2` is treated as a cylinder,
and each `item` may carry an optional `allowed_float_ratio` (default 0.25).

## Contact

* E-mail: niyongsheng@Outlook.com

## License

[Apache-2.0](LICENSE)
