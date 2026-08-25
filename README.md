# SmartPacker

![logo](./logo.png)
![Pages](https://img.shields.io/badge/r0.2.0-beta-brightgreen.svg?style=flat-square)

3D 装箱（bin packing）算法库——用于物流柜体/集装箱装载优化。

## 特性

- **垂直底部支撑** — 放置判定要求支撑物顶面恰托住底面（`y1 == y0`），而非任意投影重叠。
- **货物级「允许悬空比例」** — 每件物品可声明 `Item::allowed_float_ratio`（0..=1，默认建议 0.25）：
  底面支撑占比 ≥ `1 − allowed_float_ratio` 即合法，底面四角全部落实为兜底；`0` 必须完全支撑，`1` 不限悬空。
- **重力修正与贴靠** — `fix_point` 三轴 gap-snapping（靠地/靠壁）；`bigger_first`、`distribute_items`（多箱分发）、binding 绑定组。
- **数值稳定** — `f64` + ROUND_HALF_EVEN 舍入，EPS(1e-9) 容差；核心库零外部运行时依赖（仅 `std`），
  特性可选 `serde` / `plot`（PNG 渲染）。
- **门禁测试** — `tests/no_floating.rs` 对装箱产出断言支撑规则；`cargo run --example floating_check` 人工批量扫描。

## 快速上手

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

更多可运行示例：

```bash
cargo run --example readme_simple   # 30×10×15 箱 + 5 物品
cargo run --example cylinder_mixed  # 圆柱 + 立方混合
cargo run --example multi_bin       # 双箱 + distribute_items
cargo run --example stability       # 底部支撑两规则（允许悬空比例 / 四角支撑）
cargo run --example binding         # 绑定组
cargo run --example plot --features plot        # 渲染装箱结果 PNG
```

## 技术文档

算法内部实现（排序链、`put_item` 启发式、重力修正、底部支撑、绑定组、重心分布）、数据模型、
数值语义与测试策略的完整说明见 **[`smartpacker/doc.md`](./smartpacker/doc.md)**。

## smartpacker-server

HTTP 装箱服务（best-load 应用的服务端契约配套），默认监听 `0.0.0.0:5050`：

```bash
SMARTPACKER_ADDR=127.0.0.1:5050 cargo run -p smartpacker-server
```

| 路由 | 方法 | 行为 |
|---|---|---|
| `/` | GET | 服务横幅 |
| `/getAllData` | POST | 返回内嵌示例数据 + `Success: true` |
| `/calPacking` | POST | 入参 `{box, item, binding}`，返回 `data.{box, fitItem, unfitItem}` |

POST 路由的 GET 请求被拒绝，错误统一为 `{"Success":false,"Reason":...}`。

入参约定见 [`smartpacker-server/src/lib.rs`](./smartpacker-server/src/lib.rs) 模块文档：
`box[0].openTop[0]` 用作 `put_type`、`item` 按 `count` 展开、`type==2` 视为圆柱、
每件 `item` 可带可选 `allowed_float_ratio`（缺省 0.25）等。

## Contact Me

* E-mail: niyongsheng@Outlook.com
