# smartpacker

3D bin packing algorithm library for logistics container/box loading optimization.

## Features

- **Vertical bottom support** — placement requires the supporting surface's top to exactly sit under the item's bottom (`y1 == y0`), eliminating floating; two-tier check via support ratio or all four bottom corners.
- **Allowed float ratio** — each item declares `Item::allowed_float_ratio` (0..=1, default 0.25); `0` means full support required, `1` means no limit.
- **Placement replay** — placed items carry a per-bin chronological `step`, enabling full playback of the packing process (origin at the container's bottom-left corner, x right / y up / z forward).
- **Gravity snapping & compactness** — `fix_point` 3-axis gap-snapping, `bigger_first` sorting chain, `distribute_items` multi-bin distribution, binding groups.
- **Numeric stability** — `f64` + ROUND_HALF_EVEN rounding; optional `serde` / `plot` (PNG visualization) features.

## Quick start

```rust
use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

let mut p = Packer::new();
p.add_bin(Bin::new("example", [30.0, 10.0, 15.0], 99.0));
p.add_item(Item::new("test", "test", ItemType::Cube, [9.0, 8.0, 7.0], 1.0, 1, 100, true, "red", 0.25));
p.pack(&PackOptions::default());
```

## License

Apache-2.0
