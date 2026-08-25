# smartpacker

3D 装箱(bin packing)库——[py3dbp](https://github.com/jerry800416/3dbinpacking) 的 Rust 移植,用于物流柜体/集装箱装载优化。

## 特性

- **1:1 行为对齐 py3dbp** — `bigger_first`、`fix_point`(重力修正)、`check_stable`(稳定性双规则)、`distribute_items`(多箱分发)、binding 绑定组、重力中心分布。
- **数值语义一致** — `f64` + ROUND_HALF_EVEN 银行家舍入,`int()` 语义向零截断。
- **零运行时外部依赖**(仅 `std`);可选 `serde` / `plot`(plotters 等距 PNG 渲染)特性。

## 快速上手

```rust
use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

let mut p = Packer::new();
p.add_bin(Bin::new("example", [30.0, 10.0, 15.0], 99.0));
p.add_item(Item::new("test", "test", ItemType::Cube, [9.0, 8.0, 7.0], 1.0, 1, 100, true, "red"));
p.pack(&PackOptions::default());
```

## 文档

完整算法说明(排序链、`putItem` 启发式、重力修正、稳定性、绑定组、重心)、数据模型与数值语义见仓库 `smartpacker/doc.md`。