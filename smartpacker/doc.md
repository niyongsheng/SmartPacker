# SmartPacker crate 技术文档

> 对象主体：`smartpacker` 库 crate（位置：仓库 `smartpacker/`）。
> 面向读者：需要理解或扩展装箱算法内部实现、数值语义、模块边界的开发者。
> 相关文档：仓库根 `README.md` 提供特性总览与快速上手；本文档深入实现细节。

---

## 1. 项目概述

SmartPacker 是 3D 装箱算法库，设计目标按优先级排列：

1. **放置正确性**：已放物品不越界、不超重、满足垂直底部支撑规则（可参数化放宽），
   算法自身不得产出违反规则的放置（`tests/no_floating.rs` 门禁）。
2. **参数化悬空容忍**：每件物品可用 `Item::allowed_float_ratio` 声明允许的底面悬空占比，
   替代全局固定阈值（旧 `support_surface_ratio` 已删除）。
3. **数值稳定**：位置与尺寸统一量化到 0 位小数（ROUND_HALF_EVEN）；支撑判定在 f64
   下以 EPS(1e-9) 容差执行，避免浮点比较抖动。

特性开关（`Cargo.toml`）：

| feature | 默认 | 说明 |
|---|---|---|
| 核心 | 开 | 仅依赖 `std` |
| `serde` | 关 | `Packer`/`Bin`/`Item`/`PackOptions` 序列化往返 |
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
│   ├── packer.rs         # Bin 与 Packer（核心算法）
│   ├── auxiliary.rs      # quantize / rect_intersect / intersect
│   └── plot.rs           # （feature=plot）Painter 等距渲染
├── tests/
│   ├── invariants.rs      # proptest 属性测试（含支撑规则不变式）+ 确定性回归
│   ├── no_floating.rs     # 门禁：任何装箱产出不得违反支撑规则（随机 + 种子场景）
│   └── plots.rs           # （feature=plot）自建场景装箱效果图 PNG
└── examples/             # readme_simple / cylinder_mixed / multi_bin /
                          # stability / binding / plot / floating_check 七个示例
```

各模块职责：

| 文件 | 职责 |
|---|---|
| `lib.rs` | 公开 `Item`/`Bin`/`Packer`/`PackOptions`/`RotationType`/`Axis`/`ItemType`，`auxiliary::intersect` |
| `constants.rs` | 旋转类型常量（6 种）+ 轴向索引 + 物品类型枚举 |
| `item.rs` | 物品实体：几何、重量、允许悬空比例、旋转、量化、`dimension()` 置换 |
| `packer.rs` | 箱子实体 + 装箱器：排序、`putItem` 启发式、重力修正、底部支撑、绑定、重心、物化 |
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
    pub allowed_float_ratio: f64, // 允许底面悬空的比例 0..=1（默认建议 0.25）
    pub rotation_type: u8,     // 0..=5，见 RotationType
    pub position: [f64; 3],    // 装箱后坐标 (x, y, z)
    pub number_of_decimals: u32,
    pub step: usize,           // 放置序号（每箱内按真实放置顺序 1 起，0 = 未放置）
}
```

要点：

- `Item::new` 在 `type_of != Cube`（即圆柱）时**强制 `updown = false`**。
- `step` 在 push 进 `Bin.items` 时赋值（`put_item` / `put_corner`），受 `put_order`
  空间重排（§4.8）影响的是数组顺序而非 `step`，调用方可用 `step` 回放真实放置顺序。
- `allowed_float_ratio` 语义：放置时要求底面支撑占比 ≥ `1 − allowed_float_ratio`；
  `0` 表示必须完全支撑，`1` 表示不限制悬空（参见 §4.6）。服务端未指定时取默认值 0.25。
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
    pub check_stable: bool,        // 底部支撑检查（阈值按件取 allowed_float_ratio）
    pub put_type: i32,             // 1 一般 / 2 顶开 / 其他不排序
    pub gravity: Vec<f64>,         // 四象限重心分布（百分比），长度 4
    /* 内部（serde skip / pub(crate)） */
    fit_items: Vec<[f64; 6]>,      // 已占空间区间 [x0,x1,y0,y1,z0,z1]
    unfitted_ids: Vec<usize>,      // unfitted 的 arena 索引
}
```

要点：

- `Bin::new` 初始 `fit_items` 为**整个箱底平面**：`[[0, w, 0, h, 0, 0]]`（z ∈ [0,0]，即高度归零）
  —— 它是重力修正与支撑检查的「地面」。
- `put_item(&mut item, pivot)`：枢轴放置；失败时**不**推入 `fit_items`，但对权重超限和
  部分情况不回滚 `item.position`。
- `clear_bin()`：清空 `items` 并重置 `fit_items` 为地面平面。
- `format_numbers()`：量化 WHD 与 max_weight；**不**同步 `fit_items`。
- `Bin` 不再有全局 `support_surface_ratio`；支撑阈值一律按物品的
  `allowed_float_ratio` 逐件判定。

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
| `check_stable` | `true` | 底部支撑检查（仅 `fix_point=true` 时生效） |
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

三次**稳定**排序套叠（最后一次为主键）：

1. 体积：`bigger_first` ? 降序 : 升序
2. `loadbear` 降序
3. `level` 升序（主键）

综合优先级：**先 level（越小越先），再 loadbear（越大越先），最后体积方向**。

`sort_binding`（绑定重排）：

- 外层遍历每个绑定组，内层遍历全部物品：落入 `<group>` 的物品进该组，其余进
  `front`/`back`。
- **空绑定组直接跳过**（避免整批物品被 `min_c=0` 丢弃）；组间按**最短非空组长度**
  轮询交错（`a1,b1,a2,b2,…`），各组超长盈余进 `extra`（成为 unfitted）。

### 4.3 `pack2Bin` 轴扫描

将单件物品装入指定箱子：

```
若 有角件且箱空  →  放置 8 个角件（§4.9）
否则若 箱空     →  直接 put_item(物品原始位置)
否则            →  对 axis ∈ {x, y, z}：
                     遍历当前箱内物品作为枢轴来源
                     pivot = item.position + item.dimension()[axis]
                     put_item(pivot) 成功即 break（只试一个轴）
```

注意 `while i < bin.items.len()` 会在**循环过程中重算长度**：`put_item` 成功会
push 物品，列表增长后继续产生更多枢轴（迭代随增长列表的行为）。

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
        if fix_point { 重力修正（§4.5）+ 支撑检查（§4.6）}
        push fit_items；
        push items；
    }
    return fit;      // ⚠️ 首轮旋转决定胜负
}
// for-else：全部旋转越界 → 回滚 position
```

关键启发式：

- **首个通过边界检查的旋转类型即定胜负**（`return fit` 位于循环体末尾）：即使该旋转
  与已有物品相交导致失败，也**不再尝试后续旋转**。
- 相交判定在**重力修正之前**：物品修正下落后的最终位置可能与已有物品在几何上重叠，
  这是启发式固有输出（`tests/invariants.rs` 与 `no_floating.rs` 只断言支撑规则，
  不断言两两不重叠）。
- `fix_point` 启用时位置最终量化到 **0 位小数**；否则沿用未量化的 pivot。

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

支撑检查在修正完成后执行（见下）。

### 4.6 底部支撑检查（check_stable）

放置候选经重力修正落到 `(x, y, z)` 后，若 `check_stable` 开启，执行**垂直底部支撑检查**
（替代旧版的 z 向贴靠检查），由私有辅助函数 `bottom_support` 完成：

```
candidate 底面：[x, x+w] × [z, z+d]，高度 y0 = y（物品底面 = 放置后的 y 坐标）

若 y0 <= EPS        → 落箱底，视为全支撑（ratio = 1，四角落实）
若底面积 w*d <= EPS → 退化物品，不做限制

支撑物 = fit_items 中「顶面 y1 与 y0 之差的绝对值 <= EPS」的已占区间
        （即 y1 == y0，顶面恰好托住底面）
支撑面积 = Σ 支撑物与底面在 x/z 投影上的重叠面积
ratio   = 支撑面积 / (w × d)
四角兜底 = 底面四角 (x,z) (x+w,z) (x,z+d) (x+w,z+d) 每个角都落在
          某个 y1==y0 支撑矩形的 x/z 范围内（含 EPS 容差）

合法当且仅当：ratio + EPS >= 1 − item.allowed_float_ratio，或四角全部落实
否则 → 恢复 item.position，put_item 返回 false
```

要点：

- 支撑阈值**按件**取 `Item::allowed_float_ratio`（全局阈值字段已从
  `Bin`/`PackOptions` 删除）。允许悬空比例越高，物品可以悬挑得越多；
  `allowed_float_ratio = 0` 要求底面 100% 落实。
- 支撑物必须以**顶面恰托住底面**（y1 == y0）计，而非「存在任意投影重叠」——
  这正是旧版悬空问题的根源。
- 比较全程使用 f64 + EPS(1e-9)，不做整数截断（对比 §5 中 fix_point 自身的 i64 语义）。
- 检查嵌套在 `fix_point` 分支内（应用恒 `fix_point=true`）。

### 4.7 `gravityCenter` 四象限重心

以 `wx = w/2`、`hx = h/2` 把箱底切成 4 象限（整型边界，`wx+1` 起算右/上半）：

物件的四角落在象限内（`x_sub && y_sub`）→ 全重进该象限；
跨象限则按重叠长度/面积比例拆分重量；最终除以总重得 4 个百分比（量化 2 位小数）。

- `sum == 0`（空箱/零重）返回 `[0,0,0,0]`，避免除零。

### 4.8 `placeOrder` / `put_order`

按 `put_type` 稳定排序 `bin.items`：

- `put_type == 1`：y → z → x 三次稳定排序（稳定套叠，等效 `sort by (x, z, y)`）。
- `put_type == 2`：x → y → z（等效 `sort by (z, y, x)`）。
- 其他：不排序。

排序只改变 `bin.items` 的数组顺序，不修改 `item.step`；需要回放真实放置顺序时
应使用 `step` 排序（§3.1）。

### 4.9 角件 `corner`

- `add_corner()`：生成 8 个黑色 Cube(`corner0..7`)，边长即 `corner`，重量 0。
- `put_corner(i, item)`：放置在箱体 8 个角的位置，并登记进 `fit_items`。
- 触发条件：`bin.corner != 0` 且**箱内为空**时首要放置（首个物品放置前）。
- 角件是人工支撑件：支撑检测中不把角件作为主体复核，但参与下方物品的支撑计算。

---

## 5. 数值语义

| 语义 | 实现 | 位置 |
|---|---|---|
| `Decimal.quantize`（默认 ROUND_HALF_EVEN） | `quantize()` 银行家舍入 | `auxiliary.rs` |
| fix_point 轴间区间交集 | `i64` 截断（`as i64`，向零） | `packer.rs` `check_width/height/depth` |
| 底部支撑判定 | f64 直接比较 + EPS(1e-9)，不截断 | `packer.rs` `bottom_support` |
| 位置量化 | `quantize(x, 0)` 保留 0 位小数 | `put_item` 末尾 |

`round_half_even` 实现 `frac == 0.5` 时取偶：`floored.even() ? 不向上 : 向上`。

**精度注意**：`fix_point` 的 gap-snapping 沿用 `i64` 截断区间算术（`overlap_len`/
`overlap_count` 以 `as i64` 参与比较与长度计算）；浮点布局不同时吸附结果可能与
连续几何直觉不同，但吸附参数均为应用侧整数尺寸，实际场景无感知。底部支撑检查则
完全在 f64 上以 EPS 容差执行，保证「托底/落实」的几何判定稳定。

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

## 7. 可视化 `plot` 模块（feature=plot）

提供 `Painter::new(bin)` + `plot_box_and_items` 的等距渲染，**不追求像素级一致**
（用于人工目检装箱效果/悬空问题）。

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
- **生成图**：`cargo test --features plot --test plots -- --nocapture` 将自建场景
  （readme_simple / cylinder_mixed / multi_bin）渲染到 `<repo>/target/plots/*.png`。

---

## 8. `serde` 序列化

feature=serde 为 `Item`/`Bin`/`Packer`/`PackOptions` 派生序列化：

- `Item.type_of` → `"typeof"`；`ItemType` lowercase。
- 内部字段（`arena`/`item_ids`/`binding`/`fit_items`/`unfitted_ids`）全部 `serde(skip)`。
- 往返保证：`serde_json::to_string → from_str → to_string` 逐字节一致
  （`lib.rs::packer_json_roundtrip_lossless` 断言）。
- `Item.step` 带 `serde(default)`：0.1.x 旧载荷仍可反序列化（缺失时补 0）。

---

## 9. 测试策略

| 测试 | 位置 | 覆盖 |
|---|---|---|
| 属性测试（含支撑不变式） | `tests/invariants.rs` | proptest 随机箱/物：不越界、总重 ≤ 承重、旋转受限、物品唯一、**每件已放物品满足支撑规则**（ratio ≥ 1−allowed 或四角落实）、每箱 step 恰为 `1..=len` 的排列（真实放置顺序） |
| 支撑规则门禁 | `tests/no_floating.rs` | proptest 随机扫描 + best-load 种子场景（40HQ×2 + 20GP×3 + 474 件货物，纸箱 0.25 / 重件 0），断言零违规且种子场景不因新规则大幅回退（fitted ≥ 470/474） |
| 确定性回归 | `tests/invariants.rs` | 同输入两次运行指纹一致 |
| 边界回归 | `tests/invariants.rs` | 空绑定组、零重 gravity 返回 `[0,0,0,0]`、底部支撑四角兜底 |
| 契约 | `smartpacker-server` | 路由/响应形状 + `allowed_float_ratio` 可选出参（缺省 0.25、越界收敛 0..=1） |
| 可视化冒烟 | `tests/plots.rs` | PNG 非空 + 自建场景出图 |

门禁命令（提交前全部通过）：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --workspace
```

人工排查工具：`cargo run --example floating_check` 对种子场景与 2000 组随机扫描做
支撑检测，统计全支撑/部分悬挑/完全悬空/违规/重叠对，出现违规返回退出码 1。

---

## 10. API 使用速查

```rust
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};

let mut p = Packer::new();
p.add_bin(Bin::new("box", [30.0, 10.0, 15.0], 99.0));
// allowed_float_ratio 为 `Item::new` 最后一个参数（0 = 必须完全支撑，1 = 不限悬空；
// 缺省建议沿用服务端默认 0.25）
p.add_item(Item::new("a", "misc", ItemType::Cube, [9., 8., 7.], 1.0, 1, 100, true, "red", 0.25));
p.pack(&PackOptions { bigger_first: true, ..PackOptions::default() });

// 只读访问结果
for bin in p.bins() {
    println!("{} grav={:?}", bin.partno, bin.gravity);
    for it in &bin.items {
        println!("{} @ {:?} rot={} allowed={}", it.partno, it.position, it.rotation_type, it.allowed_float_ratio);
    }
}
```

- 装载顺序：先 `add_bin` / `add_item`，再一次性 `pack(&options)`。
- 结果读取用 `packer.bins()` 访问器。
- 圆柱体：`ItemType::Cylinder`，`updown` 自动置 false（仅 `WHD`/`HWD` 两种旋转）。

---

## 11. 性能分析

- `putItem` 相交检查为 O(箱内物品数)；`pack2Bin` 每轴遍历箱内物品作枢轴。
- 整体近似 O(物品数² × 轴数(3))；单箱无角件典型场景（几十件）为毫秒级。
- 内存为 arena 克隆模型：`pack` 结束时每件物品至多 2–3 份快照（bin.items +
  unfitted/unfit），数量级接近 Python 引用模型，无显著放大。
- 无外部运行时依赖（核心 `std` only），利于嵌入式/服务端部署。

---

## 12. 后续方向（建议）

- `pack` 重复调用语义：可考虑重置 `arena` 状态或要求 `Packer::new()`。
- 相交检查可做空间索引（如区间树）优化大规模场景。
- `fix_point` 轴区间算术的 `i64` 截断在超大尺寸场景可能溢出，可在文档中明确上界。
- `plot` 可扩展：按四象限重心上色、输出 SVG、多箱拼接。
- 支撑判定可考虑对规则外的悬挑（如超过 allowed 但仍保持重心在支撑面内）按需扩充模型。
