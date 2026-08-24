# SmartPacker

![logo](./logo.png)
![Pages](https://img.shields.io/badge/r0.1.0-beta-brightgreen.svg?style=flat-square)

> 智能装箱算法是一类用于优化物品放置在有限容器或箱子中的方式的算法。它们的目标是最大程度地减小所需的容器数量，或者最小化剩余空间，以便在物流、货物运输和仓储等领域提高效率。这些算法可以应用于各种领域，包括电子商务、供应链管理和生产制造。

## 技术文档

算法内部实现（排序链、`putItem` 启发式、重力修正、稳定性、绑定组、重心分布）、数据模型、
数值语义、可视化与测试策略的完整说明见 **[`smartpacker/doc.md`](./smartpacker/doc.md)**。

## 特性

- **py3dbp 1:1 行为对齐** — 含 `bigger_first`、`fix_point`（重力修正）、`check_stable`（稳定性双规则）、`distribute_items`（多箱分发）、binding 绑定组。
- **数值语义一致** — `f64` + `Decimal.quantize` 的 ROUND_HALF_EVEN 银行家舍入，`int()` 语义向零截断。
- **可选 `serde` / `plot` 特性**；核心库零外部运行时依赖（仅 `std`）。


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

    let whds = [
        [9.0, 8.0, 7.0],
        [4.0, 25.0, 1.0],
        [2.0, 13.0, 5.0],
        [7.0, 5.0, 4.0],
        [10.0, 5.0, 2.0],
    ];
    for (i, whd) in whds.iter().enumerate() {
        p.add_item(Item::new(
            format!("test{i}"),
            "test",
            ItemType::Cube,
            *whd,
            1.0,  // weight
            1,    // level
            100,  // loadbear
            true, // updown
            "red",
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        ..PackOptions::default()
    });

    for bin in &p.bins {
        for it in &bin.items {
            println!("{} @ {:?} rot={}", it.partno, it.position, it.rotation_type);
        }
    }
}
```

再看若干可运行的示例：

```bash
cargo run --example readme_simple   # 30×10×15 箱 + 5 物品
cargo run --example cylinder_mixed  # 圆柱 + 立方混合
cargo run --example multi_bin       # 双箱 + distribute_items
cargo run --example stability       # 稳定性两条规则（支撑面比 / 四角支撑）
cargo run --example binding         # 绑定组
cargo run --example plot --features plot   # 渲染装箱结果 PNG
```

完整 API 签名、`PackOptions` 各字段默认值与数据模型细节见[**技术文档**](#技术文档)。

## smartpacker-server

与参考 `api.py` 契约一致的 HTTP 装箱服务。

| 路由 | 方法 | 行为 |
|---|---|---|
| `/` | GET | 服务横幅 |
| `/getAllData` | POST | 返回内嵌示例数据（widadvance.json）+ `Success: true`；GET 被拒绝 |
| `/calPacking` | POST | 入参 `{box, item, binding}`，执行装箱并返回 `data.{box, fitItem, unfitItem}`；GET 被拒绝 |

```bash
# 监听默认 0.0.0.0:5050；可用环境变量覆盖
SMARTPACKER_ADDR=127.0.0.1:5050 cargo run -p smartpacker-server

# 示例
curl -X POST http://127.0.0.1:5050/getAllData
curl -X POST http://127.0.0.1:5050/calPacking \
  -H 'Content-Type: application/json' \
  -d '{"box":[{"name":"40呎超高貨櫃","WHD":[1203,235,269],"weight":26280,"openTop":[1,2],"coner":15}],
       "item":[{"name":"50_Gal_Oil_Drum","WHD":[58,35,35],"count":3,"updown":0,"type":2,"level":1,"loadbear":100,"weight":100,"color":1}],
       "binding":[["50_Gal_Oil_Drum","Wood_Table"]]}'
```

入参约定（与 api.py 一致）：`box[0]` 的 `openTop[0]` 用作 `put_type`；`item` 按 `count` 展开为 `name-N`；`type==2` 视为圆柱；`color` 按 `1..7 → red/yellow/blue/green/purple/brown/orange` 映射（替代 api.py 的随机十六进制色，见[技术文档](#技术文档)偏差表 #3）；`coner>0` 时返回 8 个黑色角件（WHD = coner 立方，weight 0）。错误统一为 `{"Success":false,"Reason":...}`。


## Contact Me

* E-mail: niyongsheng@Outlook.com
