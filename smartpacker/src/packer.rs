//! 箱体（Bin）与装箱器（Packer）。
//!
//! 算法以应用需求（best-load）驱动：放置判定要求垂直底部支撑，
//! 每件物品可用 `Item::allowed_float_ratio` 放宽允许的底面悬空占比。
//! 本模块通过「arena + 索引」方式持有物品与箱子的所有权。

use crate::auxiliary::{intersect, quantize};
use crate::constants::{ItemType, RotationType};
use crate::item::Item;
use std::fmt;

/// 几何比较容差。
const EPS: f64 = 1e-9;

/// 半开整数区间 `[st, ed)` 与 `[lo, hi)` 的交集长度。
fn overlap_len(st: i64, ed: i64, lo: i64, hi: i64) -> i64 {
    (ed.min(hi) - st.max(lo)).max(0)
}

/// 闭整数区间 `[st, ed]` 与 `[lo, hi]` 的交集元素个数。
fn overlap_count(st: i64, ed: i64, lo: i64, hi: i64) -> i64 {
    (ed.min(hi) - st.max(lo) + 1).max(0)
}

/// 装箱容器（箱子）。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bin {
    /// 唯一编号。
    pub partno: String,
    /// 宽（width）。
    pub width: f64,
    /// 高（height）。
    pub height: f64,
    /// 深（depth）。
    pub depth: f64,
    /// 最大承重。
    pub max_weight: f64,
    /// 角件边长（0 表示无角件）。
    pub corner: f64,
    /// 已装入的物品（快照）。
    pub items: Vec<Item>,
    /// 未装入物品（物化后的最终状态）。
    pub unfitted_items: Vec<Item>,
    /// 数值量化保留的小数位数。
    pub number_of_decimals: u32,
    /// 是否启用 fix_point 重力修正（配置唯一来源 `PackOptions`，pack 时注入）。
    pub(crate) fix_point: bool,
    /// 是否启用底部支撑检查（配置唯一来源 `PackOptions`，pack 时注入）。
    pub(crate) check_stable: bool,
    /// 装箱顺序类型（1 一般 / 2 顶开 / 其他不排序）。
    pub put_type: i32,
    /// 四象限重量分布（百分比）。
    pub gravity: Vec<f64>,
    /// 已占据空间区间列表，每项为 `[x0,x1,y0,y1,z0,z1]`。
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) fit_items: Vec<[f64; 6]>,
    /// 未装入物品的 arena 索引。
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) unfitted_ids: Vec<usize>,
}

impl Bin {
    /// 构造箱子，`corner` 默认 0，`put_type` 默认 1。
    ///
    /// 可通过公开字段 `corner` / `put_type` 后续修改。
    pub fn new(partno: impl Into<String>, whd: [f64; 3], max_weight: f64) -> Self {
        Bin {
            partno: partno.into(),
            width: whd[0],
            height: whd[1],
            depth: whd[2],
            max_weight,
            corner: 0.0,
            items: Vec::new(),
            unfitted_items: Vec::new(),
            number_of_decimals: 0,
            fix_point: false,
            check_stable: false,
            put_type: 1,
            gravity: Vec::new(),
            fit_items: vec![[0.0, whd[0], 0.0, whd[1], 0.0, 0.0]],
            unfitted_ids: Vec::new(),
        }
    }

    /// 对宽、高、深、承重量化（ROUND_HALF_EVEN），并记录小数位数。
    ///
    /// 注意：不更新 `fit_items`（对齐 Python 的不对称行为）。
    pub fn format_numbers(&mut self, number_of_decimals: u32) {
        self.width = quantize(self.width, number_of_decimals);
        self.height = quantize(self.height, number_of_decimals);
        self.depth = quantize(self.depth, number_of_decimals);
        self.max_weight = quantize(self.max_weight, number_of_decimals);
        self.number_of_decimals = number_of_decimals;
    }

    /// 体积。
    pub fn volume(&self) -> f64 {
        quantize(
            self.width * self.height * self.depth,
            self.number_of_decimals,
        )
    }

    /// 已装入物品的总重量（量化）。
    pub fn total_weight(&self) -> f64 {
        let sum: f64 = self.items.iter().map(|i| i.weight).sum();
        quantize(sum, self.number_of_decimals)
    }

    /// 尝试将物品放入箱子（对应 Python `putItem`）。
    ///
    /// 返回是否成功放入；对角件之外的调用会变异 `item` 的 `position`/`rotation_type`。
    pub fn put_item(&mut self, item: &mut Item, pivot: [f64; 3]) -> bool {
        let valid_item_position = item.position;
        item.position = pivot;
        // 重量检查只依赖已放物品与当前件，旋转循环内不变，提前算一次
        let total = self.total_weight();
        let n_rot = if item.updown {
            RotationType::ALL.len()
        } else {
            RotationType::NOT_UPDOWN.len()
        };
        for rot in 0..n_rot {
            item.rotation_type = rot as u8;
            let dimension = item.dimension();
            if self.width < pivot[0] + dimension[0]
                || self.height < pivot[1] + dimension[1]
                || self.depth < pivot[2] + dimension[2]
            {
                continue; // 该旋转越界，尝试下一旋转
            }
            // 第一个放得下的旋转：碰撞或成功都立即返回，不再尝试后续旋转
            if self.items.iter().any(|cur| intersect(cur, item)) {
                // 碰撞：恢复 position
                item.position = valid_item_position;
                return false;
            }
            if total + item.weight > self.max_weight {
                // 超重：不恢复 position
                return false;
            }
            if self.fix_point {
                let w = dimension[0];
                let h = dimension[1];
                let d = dimension[2];
                let mut x = pivot[0];
                let mut y = pivot[1];
                let mut z = pivot[2];
                for _ in 0..3 {
                    y = self.check_axis(
                        [x, x + w, y, y + h, z, z + d],
                        self.height,
                        (0, 1),
                        (4, 5),
                        (2, 3),
                        (2, 3),
                        2,
                    );
                    x = self.check_axis(
                        [x, x + w, y, y + h, z, z + d],
                        self.width,
                        (4, 5),
                        (2, 3),
                        (0, 1),
                        (0, 1),
                        0,
                    );
                    z = self.check_axis(
                        [x, x + w, y, y + h, z, z + d],
                        self.depth,
                        (0, 1),
                        (2, 3),
                        (4, 5),
                        (4, 5),
                        4,
                    );
                }
                if self.check_stable {
                    // 垂直底部支撑检查：比例主规则 + 底面四角兜底（见 bottom_support）。
                    // 每件物品的允许悬空比例由 Item::allowed_float_ratio 决定。
                    let (ratio, corners_ok) = self.bottom_support(x, w, z, d, y);
                    let min_support = 1.0 - item.allowed_float_ratio;
                    if ratio + EPS < min_support && !corners_ok {
                        // 稳定性失败：恢复 position
                        item.position = valid_item_position;
                        return false;
                    }
                }
                self.fit_items.push([x, x + w, y, y + h, z, z + d]);
                // 位置量化到 0 位小数（应用数据本身为整数，保持整数输出）
                item.position = [quantize(x, 0), quantize(y, 0), quantize(z, 0)];
            }
            // 真实放置序号：每箱内按时间序 1 起，push 时记录（不受 put_order 重排影响）
            item.step = self.items.len() + 1;
            self.items.push(item.clone());
            return true;
        }
        // 全部旋转越界（for-else）：恢复 position
        item.position = valid_item_position;
        false
    }

    /// 计算物品底面（位于高度 `y0`、范围 `[x, x+w] × [z, z+d]`）的支撑情况。
    ///
    /// 返回 `(支撑面积占比, 底面四角是否全部落实)`。
    /// 支撑来自 `fit_items` 中顶面（y1）恰等于 `y0` 的已占区间；
    /// `y0 == 0`（箱底）视为全支撑；退化物品（零底面积）不做限制。
    fn bottom_support(&self, x: f64, w: f64, z: f64, d: f64, y0: f64) -> (f64, bool) {
        let bottom_area = w * d;
        if bottom_area <= EPS {
            return (1.0, true);
        }
        if y0 <= EPS {
            return (1.0, true);
        }
        let mut support = 0.0;
        let mut corners = [false; 4];
        let cs = [[x, z], [x + w, z], [x, z + d], [x + w, z + d]];
        for fi in &self.fit_items {
            if (fi[3] - y0).abs() > EPS {
                continue; // 顶面必须恰好托住底面
            }
            let x_ov = (x + w).min(fi[1]) - x.max(fi[0]);
            let z_ov = (z + d).min(fi[5]) - z.max(fi[4]);
            if x_ov > EPS && z_ov > EPS {
                support += x_ov * z_ov;
            }
            for (k, c) in cs.iter().enumerate() {
                if fi[0] - EPS <= c[0]
                    && c[0] <= fi[1] + EPS
                    && fi[4] - EPS <= c[1]
                    && c[1] <= fi[5] + EPS
                {
                    corners[k] = true;
                }
            }
        }
        (support / bottom_area, corners.iter().all(|&c| c))
    }

    /// 修正物品在指定轴方向的位置（重力下落 / gap-snapping）。
    ///
    /// 三个轴向共用同一扫描逻辑，仅重叠轴对、收集区间与容器跨度不同
    /// （height: 重叠看 x/z 投影、收集 y 区间；width: 重叠看 z/y 投影、收集 x 区间；
    /// depth: 重叠看 x/y 投影、收集 z 区间）。
    #[allow(clippy::too_many_arguments)]
    fn check_axis(
        &self,
        unfix_point: [f64; 6],
        span: f64,
        pair_a: (usize, usize),
        pair_b: (usize, usize),
        collect: (usize, usize),
        top: (usize, usize),
        ret: usize,
    ) -> f64 {
        let mut v: Vec<[f64; 2]> = vec![[0.0, 0.0], [span, span]];
        for j in &self.fit_items {
            let ov_a = overlap_len(
                j[pair_a.0] as i64,
                j[pair_a.1] as i64,
                unfix_point[pair_a.0] as i64,
                unfix_point[pair_a.1] as i64,
            );
            let ov_b = overlap_len(
                j[pair_b.0] as i64,
                j[pair_b.1] as i64,
                unfix_point[pair_b.0] as i64,
                unfix_point[pair_b.1] as i64,
            );
            if ov_a != 0 && ov_b != 0 {
                v.push([j[collect.0], j[collect.1]]);
            }
        }
        let top_len = unfix_point[top.1] - unfix_point[top.0];
        v.sort_by(|a, b| a[1].total_cmp(&b[1]));
        for k in 0..v.len().saturating_sub(1) {
            if v[k + 1][0] - v[k][1] >= top_len {
                return v[k][1];
            }
        }
        unfix_point[ret]
    }

    /// 生成 8 个角件。
    fn add_corner(&self) -> Vec<Item> {
        let mut list = Vec::new();
        if self.corner != 0.0 {
            let c = quantize(self.corner, 0);
            for i in 0..8 {
                list.push(Item::new(
                    format!("corner{}", i),
                    "corner".to_string(),
                    ItemType::Cube,
                    [c, c, c],
                    0.0,
                    0,
                    0,
                    true,
                    "#000000".to_string(),
                    1.0,
                ));
            }
        }
        list
    }

    /// 放置角件。
    fn put_corner(&mut self, info: usize, mut item: Item) {
        let x = quantize(self.width - self.corner, 0);
        let y = quantize(self.height - self.corner, 0);
        let z = quantize(self.depth - self.corner, 0);
        let pos = [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, z],
            [0.0, y, z],
            [0.0, y, 0.0],
            [x, y, 0.0],
            [x, 0.0, 0.0],
            [x, 0.0, z],
            [x, y, z],
        ];
        item.position = pos[info];
        item.step = self.items.len() + 1;
        self.items.push(item);
        let c = pos[info];
        self.fit_items.push([
            c[0],
            c[0] + self.corner,
            c[1],
            c[1] + self.corner,
            c[2],
            c[2] + self.corner,
        ]);
    }

    /// 清空箱子（对应 Python `clearBin`）。
    pub fn clear_bin(&mut self) {
        self.items.clear();
        self.fit_items = vec![[0.0, self.width, 0.0, self.height, 0.0, 0.0]];
    }
}

impl fmt::Display for Bin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({}x{}x{}, max_weight:{}) vol({})",
            self.partno,
            self.width,
            self.height,
            self.depth,
            self.max_weight,
            self.volume()
        )
    }
}

/// `pack` 的配置项。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PackOptions {
    /// 大件优先。
    pub bigger_first: bool,
    /// 多箱时是否分发剩余物品。
    pub distribute_items: bool,
    /// 是否启用 fix_point 重力修正。
    pub fix_point: bool,
    /// 是否启用底部支撑检查（阈值由每件物品的 `allowed_float_ratio` 决定）。
    pub check_stable: bool,
    /// 绑定组（每组物品名）。
    pub binding: Vec<Vec<String>>,
    /// 数值量化小数位数。
    pub number_of_decimals: u32,
}

impl Default for PackOptions {
    fn default() -> Self {
        PackOptions {
            bigger_first: false,
            distribute_items: true,
            fix_point: true,
            check_stable: true,
            binding: Vec::new(),
            number_of_decimals: 0,
        }
    }
}

/// 装箱器，管理多个箱子与物品并执行装箱。
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Packer {
    /// 所有箱子。
    pub bins: Vec<Bin>,
    /// 最终未装箱物品（物化）。
    pub unfit_items: Vec<Item>,
    /// 全部原始物品（arena，唯一 owner，保留最终状态用于物化）。
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) arena: Vec<Item>,
    /// 待装物品的 arena 索引列表（对应 Python `self.items`）。
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) item_ids: Vec<usize>,
    /// 绑定组。
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) binding: Vec<Vec<String>>,
}

impl Packer {
    /// 构造空装箱器。
    pub fn new() -> Self {
        Packer::default()
    }

    /// 添加箱子。
    pub fn add_bin(&mut self, bin: Bin) {
        self.bins.push(bin);
    }

    /// 添加物品。
    pub fn add_item(&mut self, item: Item) {
        self.arena.push(item);
        self.item_ids.push(self.arena.len() - 1);
    }

    /// 返回箱子访问器（替代 Python 中不可用的 `for b in packer` 迭代协议）。
    pub fn bins(&self) -> &[Bin] {
        &self.bins
    }

    /// 执行装箱（对应 Python `Packer.pack`）。
    ///
    /// 一次性调用；重复调用行为不做保证。
    pub fn pack(&mut self, options: &PackOptions) {
        // 1. formatNumbers
        for bin in self.bins.iter_mut() {
            bin.format_numbers(options.number_of_decimals);
        }
        for item in self.arena.iter_mut() {
            item.format_numbers(options.number_of_decimals);
        }

        // 1.5 配置注入：fix_point/check_stable 一次性设置到各箱（pack2_bin 不再逐件重写）
        for bin in self.bins.iter_mut() {
            bin.fix_point = options.fix_point;
            bin.check_stable = options.check_stable;
        }

        let bigger = options.bigger_first;
        self.binding = options.binding.clone();

        // 2. bins 按体积稳定排序
        self.bins.sort_by(|a, b| {
            let va = a.volume();
            let vb = b.volume();
            if bigger {
                vb.total_cmp(&va)
            } else {
                va.total_cmp(&vb)
            }
        });

        // 3. items 排序链（level 升 -> loadbear 降 -> 体积方向 -> 插入顺序）
        self.sort_item_ids(bigger);

        // 4. binding 预处理
        let binding_active = !self.binding.is_empty();
        let mut shared_unfitted: Vec<usize> = Vec::new();
        if binding_active {
            self.sort_binding(&mut shared_unfitted);
        }

        // 5. 逐箱 pack（binding 与非 binding 唯一差异：unfitted 的归属——
        //    binding 时进跨箱共享累加器，非 binding 时进本箱）
        for idx in 0..self.bins.len() {
            let ids = self.item_ids.clone();
            for &item_id in &ids {
                let fitted = Self::pack2_bin(&mut self.bins[idx], item_id, &mut self.arena);
                if !fitted {
                    if binding_active {
                        shared_unfitted.push(item_id);
                    } else {
                        self.bins[idx].unfitted_ids.push(item_id);
                    }
                }
            }

            // 重心分布
            let gravity = Self::gravity_center(&self.bins[idx]);
            self.bins[idx].gravity = gravity;

            // distribute：移除已装箱物品（单趟 retain，取代逐个 position+remove 的 O(n²)）
            if options.distribute_items {
                let fitted: std::collections::HashSet<&str> = self.bins[idx]
                    .items
                    .iter()
                    .map(|i| i.partno.as_str())
                    .collect();
                self.item_ids
                    .retain(|&id| !fitted.contains(self.arena[id].partno.as_str()));
            }
        }

        // 6. putOrder
        self.put_order();

        // 7. 物化
        if !self.item_ids.is_empty() {
            self.unfit_items = self
                .item_ids
                .iter()
                .map(|&id| self.arena[id].clone())
                .collect();
            self.item_ids.clear();
        } else if binding_active {
            self.unfit_items = shared_unfitted
                .iter()
                .map(|&id| self.arena[id].clone())
                .collect();
        } else {
            self.unfit_items.clear();
        }
        for bin in self.bins.iter_mut() {
            if binding_active {
                bin.unfitted_items = shared_unfitted
                    .iter()
                    .map(|&id| self.arena[id].clone())
                    .collect();
            } else {
                bin.unfitted_items = bin
                    .unfitted_ids
                    .iter()
                    .map(|&id| self.arena[id].clone())
                    .collect();
            }
        }
    }

    /// 排序 `item_ids`（稳定）：level 升 -> loadbear 降 -> 体积方向
    /// （三次稳定排序链等价于单次复合排序，主键 level、次 loadbear、末体积）。
    fn sort_item_ids(&mut self, bigger_first: bool) {
        let arena = &self.arena;
        self.item_ids.sort_by(|&a, &b| {
            let va = arena[a].volume();
            let vb = arena[b].volume();
            let by_volume = if bigger_first {
                vb.total_cmp(&va)
            } else {
                va.total_cmp(&vb)
            };
            arena[a]
                .level
                .cmp(&arena[b].level)
                .then(arena[b].loadbear.cmp(&arena[a].loadbear))
                .then(by_volume)
        });
    }

    /// `sortBinding`：按绑定组轮询交错排列；组内超出最小组长度的物品进入
    /// `extra`（共享累加器），未绑定物品保持原顺序追加在组序列之后。
    ///
    /// 按应用意图实现，不复刻 Python 的嵌套循环怪癖：
    /// - 属于任一组的物品只进对应组（Python 原版在遍历其他组时会把它们
    ///   重复塞进 front/back，导致同一物品在结果中重复出现）；
    /// - 未绑定物品统一排在绑定组序列之后（Python 在第一组为空时把它们提到最前）；
    /// - 空绑定组跳过（spec 偏差 #1）。
    fn sort_binding(&mut self, extra: &mut Vec<usize>) {
        let arena = &self.arena;
        // 物品名 → 所属组索引列表（一个名字可出现在多个组）
        let mut by_name: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (gi, group) in self.binding.iter().enumerate() {
            for name in group {
                by_name.entry(name.as_str()).or_default().push(gi);
            }
        }

        let mut groups: Vec<Vec<usize>> = vec![Vec::new(); self.binding.len()];
        let mut unbound: Vec<usize> = Vec::new();
        for &id in &self.item_ids {
            match by_name.get(arena[id].name.as_str()) {
                Some(gis) => {
                    for &gi in gis {
                        groups[gi].push(id);
                    }
                }
                None => unbound.push(id),
            }
        }

        // 取非空组的最小长度（spec 偏差 #1：空绑定组跳过）
        let min_c = groups
            .iter()
            .filter(|g| !g.is_empty())
            .map(|g| g.len())
            .min()
            .unwrap_or(0);

        // 组间轮询交错：每组第 i 个依次排入
        let mut sort_bind: Vec<usize> = Vec::new();
        for i in 0..min_c {
            for g in &groups {
                if !g.is_empty() {
                    sort_bind.push(g[i]);
                }
            }
        }

        // 组内超出 min_c 的物品进 extra（同一物品跨组重复出现时只进一次）
        let in_sort: std::collections::HashSet<usize> = sort_bind.iter().copied().collect();
        for g in &groups {
            for &id in g {
                if !in_sort.contains(&id) {
                    extra.push(id);
                }
            }
        }

        self.item_ids = sort_bind;
        self.item_ids.extend(unbound);
    }

    /// 将单个物品尝试装入指定箱子（对应 Python `pack2Bin`）。返回是否装入。
    fn pack2_bin(bin: &mut Bin, item_id: usize, arena: &mut [Item]) -> bool {
        if bin.corner != 0.0 && bin.items.is_empty() {
            let corners = bin.add_corner();
            for (i, c) in corners.into_iter().enumerate() {
                bin.put_corner(i, c);
            }
            // 落入轴循环（角件作为 pivot 来源）
        } else if bin.items.is_empty() {
            let pos = arena[item_id].position;
            return bin.put_item(&mut arena[item_id], pos);
        }

        let mut fitted = false;
        for axis in 0..3usize {
            let mut i = 0;
            while i < bin.items.len() {
                let pivot = {
                    let ib = &bin.items[i];
                    let d = ib.dimension();
                    let p = ib.position;
                    match axis {
                        0 => [p[0] + d[0], p[1], p[2]],
                        1 => [p[0], p[1] + d[1], p[2]],
                        _ => [p[0], p[1], p[2] + d[2]],
                    }
                };
                if bin.put_item(&mut arena[item_id], pivot) {
                    fitted = true;
                    break;
                }
                i += 1;
            }
            if fitted {
                break;
            }
        }
        fitted
    }

    /// 四象限重心分布（对应 Python `gravityCenter`）。返回 4 个百分比。
    fn gravity_center(bin: &Bin) -> Vec<f64> {
        let w = bin.width as i64;
        let h = bin.height as i64;
        let wx = w / 2;
        let hx = h / 2;

        let mut acc = [0.0f64; 4];
        for item in &bin.items {
            let x_st = item.position[0] as i64;
            let y_st = item.position[1] as i64;
            // 旋转后的右/上缘：dimension() 按 rotation_type 置换（唯一事实源）
            let d = item.dimension();
            let x_ed = (item.position[0] + d[0]) as i64;
            let y_ed = (item.position[1] + d[1]) as i64;
            let wt = item.weight as i64 as f64;

            for j in 0..4usize {
                let (xlo, xhi, ylo, yhi) = match j {
                    0 => (0, wx, 0, hx),
                    1 => (wx + 1, w, 0, hx),
                    2 => (0, wx, hx + 1, h),
                    _ => (wx + 1, w, hx + 1, h),
                };
                let x_sub = x_st >= xlo && x_ed <= xhi;
                let y_sub = y_st >= ylo && y_ed <= yhi;
                let x_int = overlap_count(x_st, x_ed, xlo, xhi);
                let y_int = overlap_count(y_st, y_ed, ylo, yhi);
                let opp = if j < 2 { j + 2 } else { j - 2 };

                if x_sub && y_sub {
                    acc[j] += wt;
                    break;
                } else if x_sub && !y_sub && y_int != 0 {
                    let y = y_int as f64 / (y_ed - y_st) as f64 * wt;
                    acc[j] += y;
                    acc[opp] += wt - y;
                    break;
                } else if !x_sub && y_sub && x_int != 0 {
                    let x = x_int as f64 / (x_ed - x_st) as f64 * wt;
                    acc[j] += x;
                    acc[opp] += wt - x;
                    break;
                } else if !x_sub && !y_sub && y_int != 0 && x_int != 0 {
                    let all = (y_ed - y_st) as f64 * (x_ed - x_st) as f64;
                    let y = overlap_count(y_st, y_ed, 0, hx) as f64;
                    let y_2 = (y_ed - y_st) as f64 - y;
                    let x = overlap_count(x_st, x_ed, 0, wx) as f64;
                    let x_2 = (x_ed - x_st) as f64 - x;
                    acc[0] += x * y / all * wt;
                    acc[1] += x_2 * y / all * wt;
                    acc[2] += x * y_2 / all * wt;
                    acc[3] += x_2 * y_2 / all * wt;
                    break;
                }
            }
        }

        let sum: f64 = acc.iter().sum();
        if sum == 0.0 {
            return vec![0.0, 0.0, 0.0, 0.0];
        }
        acc.iter().map(|&v| quantize(v / sum * 100.0, 2)).collect()
    }

    /// `putOrder`：按 `put_type` 排序各箱 items。
    /// 原为多次稳定排序，末次为主键；等价合并为单次复合排序
    /// （put_type 2: z 主/y 次/x 末;put_type 1: x 主/z 次/y 末）。
    fn put_order(&mut self) {
        for bin in self.bins.iter_mut() {
            if bin.put_type == 2 {
                bin.items.sort_by(|a, b| {
                    a.position[2]
                        .total_cmp(&b.position[2])
                        .then(a.position[1].total_cmp(&b.position[1]))
                        .then(a.position[0].total_cmp(&b.position[0]))
                });
            } else if bin.put_type == 1 {
                bin.items.sort_by(|a, b| {
                    a.position[0]
                        .total_cmp(&b.position[0])
                        .then(a.position[2].total_cmp(&b.position[2]))
                        .then(a.position[1].total_cmp(&b.position[1]))
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(whd: [f64; 3], weight: f64) -> Item {
        Item::new(
            "p",
            "n",
            ItemType::Cube,
            whd,
            weight,
            1,
            100,
            true,
            "red",
            0.25,
        )
    }

    #[test]
    fn bin_new_sets_defaults() {
        let b = Bin::new("box", [5.0, 4.0, 3.0], 100.0);
        assert_eq!(b.partno, "box");
        assert_eq!((b.width, b.height, b.depth), (5.0, 4.0, 3.0));
        assert_eq!(b.max_weight, 100.0);
        assert_eq!(b.corner, 0.0);
        assert_eq!(b.put_type, 1);
        assert_eq!(b.number_of_decimals, 0);
        assert!(!b.fix_point && !b.check_stable);
        assert!(b.items.is_empty() && b.unfitted_items.is_empty());
        // 初始占据：整个箱底平面。
        assert_eq!(b.fit_items, vec![[0.0, 5.0, 0.0, 4.0, 0.0, 0.0]]);
    }

    #[test]
    fn bin_format_numbers_quantizes_round_half_even() {
        let mut b = Bin::new("box", [589.8, 243.8, 259.1], 85.12);
        b.format_numbers(0);
        assert_eq!((b.width, b.height, b.depth), (590.0, 244.0, 259.0));
        assert_eq!(b.max_weight, 85.0);
        assert_eq!(b.number_of_decimals, 0);
    }

    #[test]
    fn bin_volume_and_total_weight() {
        let mut b = Bin::new("box", [5.0, 4.0, 3.0], 100.0);
        assert_eq!(b.volume(), 60.0);
        assert_eq!(b.total_weight(), 0.0);

        b.items.push(cube([2.0, 2.0, 2.0], 1.0));
        b.items.push(cube([2.0, 2.0, 2.0], 2.0));
        assert_eq!(b.total_weight(), 3.0);
    }

    #[test]
    fn bin_put_item_fits_and_sets_position() {
        let mut b = Bin::new("box", [10.0, 10.0, 10.0], 100.0);
        let mut it = cube([5.0, 5.0, 5.0], 1.0);
        assert!(b.put_item(&mut it, [0.0, 0.0, 0.0]));
        assert_eq!(b.items.len(), 1);
        assert_eq!(it.position, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn bin_put_item_too_big_rejected() {
        let mut b = Bin::new("box", [10.0, 10.0, 10.0], 100.0);
        let mut it = cube([11.0, 11.0, 11.0], 1.0);
        assert!(!b.put_item(&mut it, [0.0, 0.0, 0.0]));
        assert!(b.items.is_empty());
    }

    #[test]
    fn bin_put_item_overweight_rejected() {
        let mut b = Bin::new("box", [10.0, 10.0, 10.0], 5.0);
        b.fix_point = false;
        let mut it = cube([2.0, 2.0, 2.0], 6.0);
        assert!(!b.put_item(&mut it, [0.0, 0.0, 0.0]));
        assert!(b.items.is_empty());
    }

    #[test]
    fn bin_clear_resets_fit_items() {
        let mut b = Bin::new("box", [10.0, 10.0, 10.0], 100.0);
        let mut it = cube([5.0, 5.0, 5.0], 1.0);
        assert!(b.put_item(&mut it, [0.0, 0.0, 0.0]));
        assert!(!b.items.is_empty());

        b.clear_bin();
        assert!(b.items.is_empty());
        assert_eq!(b.fit_items, vec![[0.0, 10.0, 0.0, 10.0, 0.0, 0.0]]);
    }

    #[test]
    fn bin_display_shows_identity_and_volume() {
        let b = Bin::new("example", [30.0, 10.0, 15.0], 99.0);
        let s = format!("{b}");
        assert!(s.contains("example"), "display: {s}");
        assert!(s.contains("30"), "display should include width: {s}");
        assert!(
            s.contains("vol(4500)"),
            "display should include volume: {s}"
        );
    }

    #[test]
    fn pack_options_defaults() {
        let o = PackOptions::default();
        assert!(!o.bigger_first);
        assert!(o.distribute_items);
        assert!(o.fix_point);
        assert!(o.check_stable);
        assert!(o.binding.is_empty());
        assert_eq!(o.number_of_decimals, 0);
    }

    /// 构造开启重力修正与稳定性检查的箱子。
    fn stable_bin() -> Bin {
        let mut b = Bin::new("box", [10.0, 10.0, 10.0], 100.0);
        b.fix_point = true;
        b.check_stable = true;
        b
    }

    #[test]
    fn stable_rejects_partial_support_when_not_allowed() {
        // 底座 5×2×5 在箱底,上面放 10×2×10:支撑比 25/100 = 0.25。
        let mut b = stable_bin();
        let mut base = cube([5.0, 2.0, 5.0], 1.0);
        assert!(b.put_item(&mut base, [0.0, 0.0, 0.0]));
        // allowed=0:要求全支撑,四角也不齐 → 拒绝
        let mut it = Item::new(
            "p1",
            "n",
            ItemType::Cube,
            [10.0, 2.0, 10.0],
            1.0,
            1,
            100,
            true,
            "red",
            0.0,
        );
        assert!(!b.put_item(&mut it, [0.0, 2.0, 0.0]));
        assert_eq!(b.items.len(), 1);
    }

    #[test]
    fn stable_accepts_when_within_allowed_float() {
        let mut b = stable_bin();
        let mut base = cube([5.0, 2.0, 5.0], 1.0);
        assert!(b.put_item(&mut base, [0.0, 0.0, 0.0]));
        // allowed=0.75:要求支撑比 >= 0.25,恰好压线 → 通过
        let mut it = Item::new(
            "p1",
            "n",
            ItemType::Cube,
            [10.0, 2.0, 10.0],
            1.0,
            1,
            100,
            true,
            "red",
            0.75,
        );
        assert!(b.put_item(&mut it, [0.0, 2.0, 0.0]));
        assert_eq!(b.items.len(), 2);
    }

    #[test]
    fn stable_accepts_anything_when_fully_allowed() {
        let mut b = stable_bin();
        let mut base = cube([5.0, 2.0, 5.0], 1.0);
        assert!(b.put_item(&mut base, [0.0, 0.0, 0.0]));
        // allowed=1:不限制悬空 → 通过
        let mut it = Item::new(
            "p1",
            "n",
            ItemType::Cube,
            [10.0, 2.0, 10.0],
            1.0,
            1,
            100,
            true,
            "red",
            1.0,
        );
        assert!(b.put_item(&mut it, [0.0, 2.0, 0.0]));
        assert_eq!(b.items.len(), 2);
    }

    #[test]
    fn stable_accepts_four_corner_support_fallback() {
        // 直接验证底部支撑兜底规则:支撑面只覆盖板面四角(中间镂空),
        // 支撑比不足但仍全部落实四角。
        // 注意:不做 put_item 集成场景——fix_point 会把垫片吸附到贴墙空隙,
        // 四角贴片的几何布置在真实装箱中由插入顺序动态确定。
        let mut b = Bin::new("box", [10.0, 10.0, 10.0], 100.0);
        // 模拟支撑层(板底 y=2,板面 x∈[0,10], z∈[0,10]):
        // 4 张贴片各 5×2 分居四角,中间镂空。
        // fit_items 记录 [x0,x1,y0,y1,z0,z1]。
        b.fit_items.push([0.0, 5.0, 0.0, 2.0, 0.0, 2.0]);
        b.fit_items.push([5.0, 10.0, 0.0, 2.0, 0.0, 2.0]);
        b.fit_items.push([0.0, 5.0, 0.0, 2.0, 8.0, 10.0]);
        b.fit_items.push([5.0, 10.0, 0.0, 2.0, 8.0, 10.0]);
        let (ratio, corners_ok) = b.bottom_support(0.0, 10.0, 0.0, 10.0, 2.0);
        assert!((ratio - 0.4).abs() < 1e-9, "ratio = {ratio:.4}");
        assert!(corners_ok, "four corner patches must satisfy the fallback");

        // 主规则不通过的样例:板面移到 z∈[3,7](四角都不落在贴片范围内),
        // 支撑比为 0,兜底也不成立。
        let (ratio2, corners2) = b.bottom_support(0.0, 10.0, 3.0, 4.0, 2.0);
        assert!((ratio2 - 0.0).abs() < 1e-9, "ratio2 = {ratio2:.4}");
        assert!(
            !corners2,
            "no support under z∈[3,7] => corners must be false"
        );
    }
}
