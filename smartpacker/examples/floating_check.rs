//! 货物悬空检测器:按「允许悬空比例」语义逐件检查垂直支撑,排查悬空问题。
//!
//! 规则(与算法 put_item 中的判定一致,判定实现见 `tests/common`):
//! - 支撑比 = Σ( y1 == y0 托底的支撑物在 x/z 投影重叠面积 ) / (w×d);
//! - 合法当且仅当 支撑比 ≥ 1 − allowed_float_ratio,或底面四角全部落实(兜底)。
//!
//! 统计分类(信息展示,`Violation` 才是门禁项):
//! - Full:支撑比 ≈ 1(含直接落箱底);
//! - Partial:0 < 支撑比 < 1 且合法(在允许范围内悬挑);
//! - Float:支撑比 ≈ 0(完全悬空;仅当 allowed_float_ratio ≥ 1 或四角兜底时合法);
//! - Violation:违反规则——算法不应产出任何 Violation,出现即退出码 1。
//!
//! 另统计放置物品间的几何重叠对(启发式固有输出,见 doc.md §4.4)。
//!
//! 检查对象:
//! 1. best-load 种子数据的真实场景(40HQ×2 + 20GP×3 + 474 件);
//! 2. 2000 组确定性随机扫描(每组随机允许悬空档位)。
//!
//! 用法:`cargo run --example floating_check`

#[path = "../tests/common/mod.rs"]
mod common;

use common::{support_stats, FLOAT_RATIOS};
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};

const EPS: f64 = 1e-9;

// ---------- 支撑检测 ----------

#[derive(Default)]
struct Stats {
    checked: usize,
    full: usize,
    partial: usize,
    float: usize,
    violations: usize,
    overlap_pairs: usize,
}

impl Stats {
    fn merge(&mut self, other: &Stats) {
        self.checked += other.checked;
        self.full += other.full;
        self.partial += other.partial;
        self.float += other.float;
        self.violations += other.violations;
        self.overlap_pairs += other.overlap_pairs;
    }

    fn line(&self, prefix: &str) -> String {
        format!(
            "{prefix}: 检查 {} 件 | 全支撑 {} | 部分悬挑 {} | 完全悬空 {} | 违反规则 {} | 重叠对 {}",
            self.checked, self.full, self.partial, self.float, self.violations, self.overlap_pairs
        )
    }
}

/// 对单箱放置结果做支撑检测,findings 追加 Violation 明细(每箱上限 max_lines 条);
/// 前 max_evidence 个违反规则得最严重的箱转储完整几何供人工复核。
fn check_bin(
    bin: &Bin,
    tag: &str,
    findings: &mut Vec<String>,
    max_lines: usize,
    stats: &mut Stats,
    evidence: &mut Vec<String>,
    max_evidence: usize,
) {
    if bin.items.is_empty() {
        return;
    }
    let n = bin.items.len();
    let mut shown = 0;
    for i in 0..n {
        let it = &bin.items[i];
        // 角件是人工支撑件,不作为主体检查(仍参与下方支撑计算)
        if it.partno.starts_with("corner") {
            continue;
        }
        let [w, h, d] = it.dimension();
        let [x, y, z] = it.position;
        if w * d <= EPS {
            continue; // 退化物品(零底面积)不检测
        }
        stats.checked += 1;

        let (ratio, corners_ok) = support_stats(it, bin);
        let legal = ratio + EPS >= 1.0 - it.allowed_float_ratio || corners_ok;

        if ratio <= EPS {
            stats.float += 1;
        } else if ratio + EPS >= 1.0 {
            stats.full += 1;
        } else {
            stats.partial += 1;
        }

        if !legal {
            stats.violations += 1;
            if shown < max_lines {
                shown += 1;
                findings.push(format!(
                    "[{tag}] bin={} item={} pos=({x},{y},{z}) dims=({w},{h},{d}) rot={} ratio={ratio:.3} allowed={:.3} corners={corners_ok} VIOLATION",
                    bin.partno, it.partno, it.rotation_type, it.allowed_float_ratio
                ));
            }
            if evidence.len() < max_evidence {
                // 转储该箱全部物品几何(按 y,x,z 排序),供人工复核成因
                let mut items: Vec<&Item> = bin
                    .items
                    .iter()
                    .filter(|i| !i.partno.starts_with("corner"))
                    .collect();
                items.sort_by(|a, b| {
                    a.position[1]
                        .partial_cmp(&b.position[1])
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(
                            a.position[0]
                                .partial_cmp(&b.position[0])
                                .unwrap_or(std::cmp::Ordering::Equal),
                        )
                        .then(
                            a.position[2]
                                .partial_cmp(&b.position[2])
                                .unwrap_or(std::cmp::Ordering::Equal),
                        )
                });
                let mut dump = format!(
                    "违规证据[{tag}] bin={} ({}x{}x{}) 违规件: {} pos=({x},{y},{z}) dims=({w},{h},{d}) rot={} ratio={ratio:.3}",
                    bin.partno, bin.width, bin.height, bin.depth, it.partno, it.rotation_type
                );
                for o in items {
                    let [ow, oh, od] = o.dimension();
                    let [ox, oy, oz] = o.position;
                    let mark = if o.partno == it.partno {
                        " <== 违规"
                    } else {
                        ""
                    };
                    dump.push_str(&format!(
                        "\n    {} pos=({ox},{oy},{oz}) dims=({ow},{oh},{od}) top_y={}{}",
                        o.partno,
                        oy + oh,
                        mark
                    ));
                }
                evidence.push(dump);
            }
        }
    }

    // 重叠对统计(严格三维相交,复用库内 intersect)
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (&bin.items[i], &bin.items[j]);
            if a.partno.starts_with("corner") && b.partno.starts_with("corner") {
                continue;
            }
            if smartpacker::intersect(a, b) {
                stats.overlap_pairs += 1;
            }
        }
    }
}

/// 执行一次装箱并对所有箱做支撑检测。
fn run_pack(
    packer: &mut Packer,
    options: &PackOptions,
    tag: &str,
    findings: &mut Vec<String>,
    evidence: &mut Vec<String>,
) -> Stats {
    packer.pack(options);
    let mut stats = Stats::default();
    for bin in packer.bins() {
        check_bin(bin, tag, findings, 20, &mut stats, evidence, 3);
    }
    stats
}

// ---------- 场景构造 ----------

/// best-load 种子数据场景(与门禁测试共享,见 tests/common)。
fn seed_packer() -> Packer {
    common::seed_packer()
}

// ---------- 确定性随机扫描 ----------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

fn random_packer(rng: &mut Rng) -> Packer {
    let mut packer = Packer::new();
    let bin_count = rng.range(1, 2);
    for i in 0..bin_count {
        let w = rng.range(100, 600) as f64;
        let h = rng.range(100, 400) as f64;
        let d = rng.range(100, 400) as f64;
        packer.add_bin(Bin::new(format!("bin{i}"), [w, h, d], 1e9));
    }
    let item_count = rng.range(5, 50);
    for i in 0..item_count {
        let w = rng.range(10, 200) as f64;
        let h = rng.range(10, 200) as f64;
        let d = rng.range(10, 200) as f64;
        let updown = rng.range(0, 1) == 1;
        let level = rng.range(1, 2) as i32;
        let loadbear = rng.range(1, 100) as i32;
        let allowed = FLOAT_RATIOS[rng.range(0, 4) as usize];
        packer.add_item(Item::new(
            format!("it{i}"),
            "misc",
            ItemType::Cube,
            [w, h, d],
            1.0,
            level,
            loadbear,
            updown,
            "red",
            allowed,
        ));
    }
    packer
}

// ---------- 主流程 ----------

fn main() {
    let mut total = Stats::default();
    let mut findings: Vec<String> = Vec::new();
    let mut evidence: Vec<String> = Vec::new();

    // 1. 种子数据真实场景
    println!("===== 1. best-load 种子数据场景 =====");
    let mut seed_default = Stats::default();
    let mut p = seed_packer();
    seed_default.merge(&run_pack(
        &mut p,
        &PackOptions::default(),
        "seed/default",
        &mut findings,
        &mut evidence,
    ));
    let mut seed_bf = Stats::default();
    let mut p = seed_packer();
    seed_bf.merge(&run_pack(
        &mut p,
        &PackOptions {
            bigger_first: true,
            ..PackOptions::default()
        },
        "seed/bigger_first",
        &mut findings,
        &mut evidence,
    ));
    println!("{}", seed_default.line("seed/app-default"));
    println!("{}", seed_bf.line("seed/app-bigger  "));
    total.merge(&seed_default);
    total.merge(&seed_bf);

    // 2. 随机扫描
    println!("\n===== 2. 随机扫描(2000 组) =====");
    let mut sweep = Stats::default();
    let mut rng = Rng(0x5eed_2026_0825);
    for t in 0..2000 {
        let bigger_first = rng.range(0, 1) == 1;
        let mut packer = random_packer(&mut rng);
        let before = sweep.violations;
        sweep.merge(&run_pack(
            &mut packer,
            &PackOptions {
                bigger_first,
                ..PackOptions::default()
            },
            &format!("sweep#{t}"),
            &mut findings,
            &mut evidence,
        ));
        if sweep.violations > before {
            println!("sweep#{t}: 新发现违规(bigger_first={bigger_first})");
        }
    }
    println!("{}", sweep.line("sweep"));
    total.merge(&sweep);

    // 汇总
    println!("\n===== 汇总 =====");
    println!("{}", total.line("总计"));
    if !findings.is_empty() {
        println!("\n违规明细(每箱最多 20 条):");
        for f in &findings {
            println!("  {f}");
        }
    }
    if !evidence.is_empty() {
        println!("\n违规证据(前 {} 个违规箱的完整几何):", evidence.len());
        for e in &evidence {
            println!("{e}");
        }
    }
    if total.violations > 0 {
        println!("\n结论:存在违反支撑规则的放置,退出码 1。");
        std::process::exit(1);
    }
    println!("\n结论:未发现违反支撑规则(退出码 0)。");
}
