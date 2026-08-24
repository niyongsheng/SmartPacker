# SmartPacker（smartpacker crate）技术文档

> 对象主体：`smartpacker` 库 crate（位置：仓库 `smartpacker/`）。
> 面向读者：需要理解或扩展装箱算法内部实现、数值语义、模块边界的开发者。
> 相关文档：仓库根 `README.md` 提供特性总览与快速上手；本文档深入实现细节。

---

## 1. 项目概述

SmartPacker 是 Python 3D 装箱库 [py3dbp](https://github.com/jerry800416/3D-bin-packing)
的 Rust 1:1 移植。设计目标按优先级排列：

1. **行为保真**：算法逐分支复刻，含原库特有的启发式（如「仅尝试首个通过边界检查的旋转
   类型」），并用 NVIDIA 黄金数据（`tests/golden/*.json`）逐字段断言输出一致。
2. **数值语义一致**：浮点位权与 Python `Decimal` / `int()` 语义对齐（银行家舍入、向零截断）。
3. **仅修复崩溃类 bug**：凡与 Python 不一致之处全部在本文档 §7「已文档化偏差」表中登记。

特性开关（`Cargo.toml`）：

| feature | 默认 | 说明 |
|---|---|---|
| 核心 | 开 | 仅依赖 `std` |
| `serde` | 关 | `Packer`/`Bin`/`Item` 序列化往返 |
| `plot` | 关 | plotters 等距投影渲染 PNG |

---

## 2. 模块结构与源码地图

```
smartpacker/
├── Cargo.toml            # 包元数据；features: serde / plot
├── src/
│   ├── lib.rs            # crate 根；pub 导出、模块声明、serde 往返测试
│   ├── constants.rs      # RotationType / Axis / ItemType
│   ├── item.rs           # Item 类型
│   ├── packer.rs         # Bin 与 Packer（核心算法，约 940 行）
│   ├── auxiliary.rs      # quantize / rect_intersect / intersect
│   └── plot.rs           # （feature=plot）Painter 等距渲染
├── tests/
│   ├── golden/*.json     # py3dbp 生成的黄金基准数据（只读输入）
│   ├── golden_parity.rs  # 黄金对齐测试：逐字段断言与 Python 输出一致
│   ├── invariants.rs     # proptest 属性测试 + 确定性 + R7/R8 回归
│   └── plots.rs          # （feature=plot）渲染 golden 场景装箱效果图 PNG
└── examples/             # readme_simple / cylinder_mixed / multi_bin /
                          # stability / binding / plot 六大示例
```

各模块职责：

| 文件 | 职责 |
|---|---|
| `lib.rs` | 公开 `Item`/`Bin`/`Packer`/`PackOptions`/`RotationType`/`Axis`/`ItemType`，`auxiliary::intersect` |
| `constants.rs` | 旋转类型常量（6 种）+ 轴向索引 + 物品类型枚举 |
| `item.rs` | 物品实体：几何、重量、旋转、量化、`dimension()` 置换 |
| `packer.rs` | 箱子实体 + 装箱器：排序、`putItem` 启发式、重力修正、稳定性、绑定、重心、物化 |
| `auxiliary.rs` | 数值量化与几何相交判定 |
| `plot.rs` | 可视化；`Painter::new(bin).plot_box_and_items(...)` |

---

## 3. 数据模型

### 3.1 `Item`

```rust
pub struct Item {
    pub partno: String,        // 唯一编号（PN）
    pub name: String,          // 物品类型名称（binding 按此分组）
    pub type_of: ItemType,     // Cube | Cylinder
    pub width / height / depth: f64,
    pub weight: f64,
    pub level: i32,            // 优先级，越小越先装
    pub loadbear: i32,         // 承重能力，越大越先装
    pub updown: bool,          // 是否允许倒放
    pub color: String,         // 显示色（命名色或 #RRGGBB）
    pub rotation_type: u8,     // 0..=5，见 RotationType
    pub position: [f64; 3],    // 装箱后坐标 (x, y, z)
    pub number_of_decimals: u32,
}
```

要点：

- `Item::new` 在 `type_of != Cube`（即圆柱）时**强制 `updown = false`**，对齐 Python。
- `dimension()` 按当前 `rotation_type` 置换 `[w, h, d]` → `[x, y, z]`（6 种排列，见 §3.4）。
- `max_area()`：`updown` 时取最大两维之积，否则取 `width × height`。
- `format_numbers()` 量化为 ROUND_HALF_EVEN（§5）。

### 3.2 `Bin`

```rust
pub struct Bin {
    pub partno: String,
    pub width / height / depth: f64,
    pub max_weight: f64,
    pub corner: f64,               // 角件边长；0 表示无角件
    pub items: Vec<Item>,          // 装箱结果快照
    pub unfitted_items: Vec<Item>, // 本箱未装入（物化副本）
    pub number_of_decimals: u32,
    pub fix_point: bool,           // 运行时由 PackOptions 注入
    pub check_stable: bool,
    pub support_surface_ratio: f64,
    pub put_type: i32,             // 1 一般 / 2 顶开 / 其他不排序
    pub gravity: Vec<f64>,         // 四象限重心分布（百分比），长度 4
    /* 内部（serde skip / pub(crate)） */
    fit_items: Vec<[f64; 6]>,      // 已占空间区间 [x0,x1,y0,y1,z0,z1]
    unfitted_ids: Vec<usize>,      // unfitted 的 arena 索引
}
```

要点：

- `Bin::new` 初始 `fit_items` 为**整个箱底平面**：`[[0, w, 0, h, 0, 0]]`（z ∈ [0,0]，即高度归零）
  —— 它是重力修正与稳定性检查的「地面」。
- `put_item(&mut item, pivot)`：枢轴放置；失败时**不**推入 `fit_items`，但对权重超限和
  部分情况不回滚 `item.position`（对齐 Python 的原始行为）。
- `clear_bin()`：清空 `items` 并重置 `fit_items` 为地面平面。
- `format_numbers()`：量化 WHD 与 max_weight；**不**同步 `fit_items`（对齐 Python 不对称行为）。

### 3.3 `Packer`

```rust
pub struct Packer {
    pub bins: Vec<Bin>,
    pub unfit_items: Vec<Item>,        // 最终未装物品（物化副本）
    /* 内部（serde skip / pub(crate)） */
    arena: Vec<Item>,                  // 全部原始物品，item 的唯一 owner
    item_ids: Vec<usize>,              // 待装物品的 arena 索引
    binding: Vec<Vec<String>>,         // 绑定组
}
```

`pack()` 在 crate 内只暴露为**一次性调用**；重复调用不保证行为。

### 3.4 `constants.rs`

- `RotationType` 是**原始索引常量**（非穷举枚举），共 6 种：
  `RT_WHD(0)`、`RT_HWD(1)`、`RT_HDW(2)`、`RT_DHW(3)`、`RT_DWH(4)`、`RT_WDH(5)`，
  与 `Item::dimension()` 的置换一一对应。

  | RT | (x, y, z) |
  |---|---|
  | 0 RT_WHD | (w, h, d) |
  | 1 RT_HWD | (h, w, d) |
  | 2 RT_HDW | (h, d, w) |
  | 3 RT_DHW | (d, h, w) |
  | 4 RT_DWH | (d, w, h) |
  | 5 RT_WDH | (w, d, h) |

  - `RotationType::ALL` = 全部 6 种；`NOT_UPDOWN` = 仅 `[RT_WHD, RT_HWD]`。
- `Axis::WIDTH/HEIGHT/DEPTH` = 0/1/2。
- `ItemType::{Cube, Cylinder}`，serde 序列化为 `"cube"/"cylinder"`（lowercase）。

### 3.5 `PackOptions`（默认值与语义）

| 字段 | 默认 | 语义 |
|---|---|---|
| `bigger_first` | `false` | 排序方向：true 为体积降序（大件优先） |
| `distribute_items` | `true` | 多箱时把已装物品从待装队列剔除，供后续箱分发 |
| `fix_point` | `true` | 放置后做重力修正（靠地/靠壁下落） |
| `check_stable` | `true` | 稳定性双规则（仅 `fix_point=true` 时生效） |
| `support_surface_ratio` | `0.75` | 底部支撑面积占比阈值 |
| `binding` | `[]` | 绑定组，每组是一列物品 `name` |
| `number_of_decimals` | `0` | 数值量化位数（ROUND_HALF_EVEN） |

---

## 4. 装箱算法

### 4.1 整体流程（`Packer::pack`）

```
1. format_numbers        所有 bin / item 量化
2. sort bins             按体积稳定排序（bigger_first 决定升降序）
3. sort items            排序链 plate（见 §4.2）
4. binding 预处理        若启用 binding：sort_binding 重排 item_ids
5. 逐箱 pack2Bin         对每个箱子按当前 item_ids 逐一尝试放置
   a. pack2Bin           三个轴扫描枢轴（§4.3）
   b. gravity            计算四象限重心
   c. distribute_items   剔除本箱已装，供后续箱继续装
6. put_order             按 put_type 重排 bin.items
7. 物化                  unfitted / unfit_items 从 arena 克隆出来
```

### 4.2 排序链（`sort_item_ids`）

三次**稳定**排序套叠（最后一次为主键），完全对应 Python 连续 `sorted()`：

1. 体积：`bigger_first` ? 降序 : 升序
2. `loadbear` 降序
3. `level` 升序（主键）

综合优先级：**先 level（越小越先），再 loadbear（越大越先），最后体积方向**。

`sort_binding`（绑定重排）：
- 外层遍历每个绑定组，内层遍历全部物品：落入 `<group>` 的物品进该组，其余进
  `front`/`back`（原库注释 `item.name not in self.binding` 恒为真——比较对象是
  「积分组列表」与物品名，类型永不等）。
- 组间按**最短非空组长度**轮询交错（`a1,b1,a2,b2,…`），各组超长盈余进
  `extra`（成为 unfitted）。
- **偏差 #1**：跳过空绑定组，避免整批物品被 `min_c=0` 丢弃。

### 4.3 `pack2Bin` 轴扫描

将单件物品装入指定箱子：

```
若 有角件且箱空  →  放置 8 个角件（§4.10）
否则若 箱空     →  直接 put_item(物品原始位置)
否则            →  对 axis ∈ {x, y, z}：
                     遍历当前箱内物品作为枢轴来源
                     pivot = item.position + item.dimension()[axis]
                     put_item(pivot) 成功即 break（只试一个轴）
```

注意 `while i < bin.items.len()` 会在**循环过程中重算长度**：`put_item` 成功会
push 物品，列表增长后继续产生更多枢轴（复刻 Python 迭代随增长列表的行为）。

### 4.4 `putItem` 启发式（核心）

```rust
item.position = pivot;
n_rot = updown ? 6 : 2;
for rot in 0..n_rot {
    set rotation_type;
    越界检查（pivot + dim <= bin 尺寸）  → continue 换下一旋转；
    相交检查：与 bin.items 逐个 intersect → 不通过则 fit=false；
    if fit {
        超重检查：total_weight + weight > max_weight → return false（不回滚 pos）
        if fix_point { 重力修正（§4.5）+ 稳定性（§4.6）}
        push fit_items；
        push items；
    }
    return fit;      // ⚠️ 关键启发式
}
// for-else：全部旋转越界 → 回滚 position
```

关键启发式（忠实复刻原库的行为，非本移植新增）：

- **首个通过边界检查的旋转类型即定胜负**（`return fit` 位于循环体末尾）：即使该旋转
  与已有物品相交导致失败，也**不再尝试后续旋转**。
- 相交判定在**重力修正之前**：物品修正下落后的最终位置可能与已有物品在几何上重叠，
  这是原库启发式的固有输出（非移植缺陷，`tests/invariants.rs` 对此不做断言）。
- `fix_point` 启用时位置最终量化到 **0 位小数**（对齐 Python `set2Decimal` 默认值）；
  否则沿用未量化的 pivot。

### 4.5 `fix_point` 重力修正

对候选区间 `[x, x+w, y, y+h, z, z+d]` 连续迭代 3 轮：

```
y = check_height(...)   // 在 x*z 有重叠的已占区间集合里找可容纳的空隙落下
x = check_width(...)
z = check_depth(...)
```

每个 `check_*` 的做法一致：

1. 区间表初始化为 `{[0,0], [lim, lim]}`（lim 为该轴箱体尺寸）。
2. 收集**在另两轴上与候选投影重叠**的已占区间的该轴范围。
3. 按区间上界升序排列，寻找第一个满足 `next.lower - cur.upper >= 物品该轴长度`
   的间隙，把物品「压」到 `cur.upper`；找不到则停在原始坐标。

`check_stable` 在修正完成后执行（见下）。

### 4.6 `check_stable` 稳定性双规则

仅当 `z == 已占区间上界 fi[5]` 时该区间视为支撑面。物品底面积 `lower = w*h`：

- **规则一（支撑面比）**：统计与物品底面积相交的支撑区间面积
  `support_area_upper`；若 `support_area_upper / lower < support_surface_ratio`，
  触发规则二复查。
- **规则二（四角支撑）**：底边四角 `{(x,y),(x+w,y),(x,y+h),(x+w,y+h)}` 若存在任一
  未被支撑区间覆盖 → 稳定性失败，回滚 `item.position`，`putItem` 返回 false。
- **偏差 #7（崩溃修复）**：底面积 `w*h == 0`（退化尺寸）时跳过顶点检查，避免被零除。

> 稳定性与重力的区间算术统一走 **`i64`（截断）整数**，见 §5。

### 4.7 `gravityCenter` 四象限重心

以 `wx = w/2`、`hx = h/2` 把箱底切成 4 象限（整型边界，`wx+1` 起算右/上半）：

物件的四角落在象限内（`x_sub && y_sub`）→ 全重进该象限；
跨象限则按重叠长度/面积比例拆分重量；最终除以总重得 4 个百分比（量化 2 位小数）。

- `sum == 0`（空箱/零重）返回 `[0,0,0,0]`（**偏差 #2**，修复除零崩溃）。

### 4.8 `placeOrder` / `put_order`

按 `put_type` 稳定排序 `bin.items`：

- `put_type == 1`：y → z → x 三次稳定排序（稳定套叠，等效 `sort by (x, z, y)`）。
- `put_type == 2`：x → y → z（等效 `sort by (z, y, x)`）。
- 其他：不排序。

### 4.9 角件 `corner`

- `add_corner()`：生成 8 个黑色 Cube(`corner0..7`)，边长即 `corner`，重量 0。
- `put_corner(i, item)`：放置在箱体 8 个角的位置，并登记进 `fit_items`。
- 触发条件：`bin.corner != 0` 且**箱内为空**时首要放置（首个物品放置前）。

---

## 5. 数值语义

| Python 语义 | Rust 实现 | 位置 |
|---|---|---|
| `Decimal.quantize`（默认 ROUND_HALF_EVEN） | `quantize()` 银行家舍入 | `auxiliary.rs` |
| `int(x)` 向零截断 | `as i64`（Rust 向零截断） | 遍布 `packer.rs` 稳定性/重心计算 |
| `set2Decimal` 默认 0 位 | 位置量化 0 位 | `putItem` 末尾 |

`round_half_even` 实现 `frac == 0.5` 时取偶：`floored.even() ? 不向上 : 向上`。

**精度注意**：`overlap_len`/`overlap_count`/稳定性/重心全部把 `f64` 截断到 `i64`
再做比较与长度计算——这是对齐 Python `int()` 语义的刻意选择；浮点布局不同时结果
可能与直觉（连续几何）不同，但保证与 py3dbp 输出一致。

---

## 6. arena + 索引设计

源库 Python 依靠对象引用共享 `Item`（同一对象被多个 `bin.items` / `unfitted_items`
列表引用，`position`/`rotation_type` 是就地变异）。Rust 借用规则不允许这种别名，
因此选用：

- **arena**：`Packer` 内部 `Vec<Item>` 持有每件物品的**唯一 owner**；
- **索引**：`item_ids` / `bin.unfitted_ids` 保存 `usize`；
- **物化**：`pack` 结束阶段把最终状态克隆成公开的 `Vec<Item>`（`items` /
  `unfitted_items` / `unfit_items`）。

代价是：公开快照是克隆，`position`/`rotation_type` 的变异发生在 arena 元素上，须通过
`Packer` 内部索引访问；普通只读用户无感知。

---

## 7. 与 Python 的已文档化偏差

与算法实现直接相关的三条（#1/#2/#7）均以失败终止为界修复，不影响成功路径的输出一致性
（黄金测试约束）；其余为服务端/可视化/迭代协议类差异。

| # | 偏差 | 原因 |
|---|------|------|
| 1 | binding 空组跳过而非全丢弃 | 空组会致 `min_c=0` 丢弃全部物品，修复 |
| 2 | gravity 零重量返回 `[0,0,0,0]` 而非崩溃 | 修复除零 |
| 3 | 服务端 color 采用 api.md 1-7 命名色映射；越界回退 `#808080` | api.py 随机色与 api.md 文档矛盾，取文档语义 |
| 4 | 服务端 JSON 解析替代 `eval()` | 安全修复 |
| 5 | Painter 改为 plotters 等距渲染 PNG | matplotlib 不可移植，对齐 API 意图而非像素 |
| 6 | Packer 迭代协议（Python `for b in packer` 实际不可用） | 以 `packer.bins()` 访问器替代 |
| 7 | `check_stable` 中物品底面积为 0（退化尺寸）时跳过顶点检查 | 对齐 py3dbp 主分支的守卫性修复（Python 旧版会除零崩溃） |

算法相关偏差 #1/#2/#7 的实现位置：`sort_binding`（§4.2）、`gravity_center`（§4.7）、
`check_stable`（§4.6）。

---

## 8. 可视化 `plot` 模块（feature=plot）

对齐 Python `Painter` 的 API 意图（`Painter::new(bin)` + `plot_box_and_items`），
**不追求像素级一致**（matplotlib 不可移植，见偏差 #5）。

```rust
Painter::new(&bin)
    .plot_box_and_items(title, alpha /*0.8*/, write_num /*true*/, fontsize /*14*/, "out.png")
```

- **投影**：等距（isometric）`(x - z)·cos30°, y - (x + z)·sin30°`，`equal_aspect`
  维持 canvas 纵横比并留白 12%，画布固定 1024×768。
- **箱体**：黑色线框（12 条棱）。
- **立方体**：顶 + 两个可见侧面用 `with_alpha` 混白填充；黑色线框；可选 `partno` 标注。
- **圆柱体**：上下椭圆 + 侧面多边形示意 + 轮廓。
- **颜色**：`rgb_from_name` 支持 `#RRGGBB` 与命名色；未知名走确定性哈希（同名同色）。
- **生成图**：`cargo test --features plot --test plots -- --nocapture` 将
  `tests/golden/` 全部场景渲染到 `<repo>/target/plots/*.png`。

---

## 9. `serde` 序列化

feature=serde 为 `Item`/`Bin`/`Packer`/`PackOptions` 派生序列化：

- `Item.type_of` → `"typeof"`；`ItemType` lowercase。
- 内部字段（`arena`/`item_ids`/`binding`/`fit_items`/`unfitted_ids`）全部 `serde(skip)`。
- 往返保证：`serde_json::to_string → from_str → to_string` 逐字节一致
  （`lib.rs::packer_json_roundtrip_lossless` 断言）。

---

## 10. 测试策略

| 测试 | 位置 | 覆盖 |
|---|---|---|
| 黄金对齐 | `tests/golden_parity.rs` | 16 个 py3dbp 基准场景，逐字段断言（partno、坐标、旋转、gravity、unfitted 全等） |
| 属性测试 | `tests/invariants.rs` | proptest 随机箱/物：不越界、总重 ≤ 承重、旋转受限、物品守恒；不断言两两不重叠（启发式固有） |
| 确定性 | `tests/invariants.rs` | 同输入两次运行指纹一致 |
| R7/R8 回归 | `tests/invariants.rs` | 空绑定组、零重 gravity |
| 契约 | `smartpacker-server` | 路由/响应形状对齐参考 `api.py` |
| 可视化冒烟 | `src/plot.rs` 单元 + `tests/plots.rs` | PNG 非空 + 16 场景出图 |

行覆盖率（`cargo llvm-cov --workspace --all-features`，`smartpacker/src/*.rs` 均 ≥90%）：
`auxiliary.rs` 100%，`lib.rs` 100%，`item.rs` 99.2%，`packer.rs` 95.4%。

门禁命令（提交前全部通过）：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --workspace
```

---

## 11. API 使用速查

```rust
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};

let mut p = Packer::new();
p.add_bin(Bin::new("box", [30.0, 10.0, 15.0], 99.0));
p.add_item(Item::new("a", "misc", ItemType::Cube, [9., 8., 7.], 1.0, 1, 100, true, "red"));
p.pack(&PackOptions { bigger_first: true, ..PackOptions::default() });

// 只读访问结果
for bin in p.bins() {
    println!("{} grav={:?}", bin.partno, bin.gravity);
    for it in &bin.items {
        println!("{} @ {:?} rot={}", it.partno, it.position, it.rotation_type);
    }
}
```

- 装载顺序：先 `add_bin` / `add_item`，再一次性 `pack(&options)`。
- 结果读取用 `packer.bins()` 访问器（替代 Python 的 `for b in packer`，见偏差 #6）。
- 圆柱体：`ItemType::Cylinder`，`updown` 自动置 false（仅 `WHD`/`HWD` 两种旋转）。

---

## 12. 性能分析

- `putItem` 相交检查为 O(箱内物品数)；`pack2Bin` 每轴遍历箱内物品作枢轴。
- 整体近似 O(物品数² × 轴数(3))；单箱无角件典型场景（几十件）为毫秒级。
- 内存为 arena 克隆模型：`pack` 结束时每件物品至多 2–3 份快照（bin.items +
  unfitted/unfit），数量级接近 Python 引用模型，无显著放大。
- 无外部运行时依赖（核心 `std` only），利于嵌入式/服务端部署。

## 13. 后续方向（建议）

- `pack` 重复调用语义：可考虑重置 `arena` 状态或要求 `Packer::new()`。
- 相交检查可做空间索引（如区间树）优化大规模场景。
- `overlap_len` 的 `i64` 截断在超大尺寸场景可能溢出，可在文档中明确上界。
- `plot` 可扩展：按四象限重心上色、输出 SVG、多箱拼接。