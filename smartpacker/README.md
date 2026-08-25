# smartpacker

3D 装箱(bin packing)库——以应用需求(best-load)驱动,用于物流柜体/集装箱装载优化。

## 特性

- **垂直底部支撑** — 放置判定要求支撑物顶面恰托住底面(y1 == y0),而非任意投影重叠;支撑面积比与四角支撑两级判定。
- **货物级「允许悬空比例」** — 每件物品可声明 `Item::allowed_float_ratio`(0..=1,默认建议 0.25):`支撑比 ≥ 1 − allowed_float_ratio` 即合法,四角全部落实为兜底;`0` 必须完全支撑,`1` 不限悬空。
- **重力修正与贴靠** — `fix_point` 三轴 gap-snapping、`bigger_first`(大件优先)、`distribute_items`(多箱分发)、binding 绑定组、四象限重心分布。
- **数值稳定** — `f64` + ROUND_HALF_EVEN 银行家舍入;支撑判定以 EPS(1e-9) 容差执行;位置量化到 0 位小数。
- **零运行时外部依赖**(仅 `std`);可选 `serde` / `plot`(plotters 等距 PNG 渲染)特性。

## 快速上手

```rust
use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

let mut p = Packer::new();
p.add_bin(Bin::new("example", [30.0, 10.0, 15.0], 99.0));
p.add_item(Item::new("test", "test", ItemType::Cube, [9.0, 8.0, 7.0], 1.0, 1, 100, true, "red", 0.25));
p.pack(&PackOptions::default());
```

支撑门禁与人工排查:库内在 `tests/no_floating.rs` 对装箱产出做支撑规则断言;
工具可运行 `cargo run --example floating_check` 做批量扫描。

## 文档

完整算法说明(排序链、`putItem` 启发式、重力修正、底部支撑、绑定组、重心)、数据模型与数值语义见仓库 `smartpacker/doc.md`。
