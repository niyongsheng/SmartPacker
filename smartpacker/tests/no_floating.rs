//! 门禁测试:装箱产出不得违反「允许悬空比例」支撑规则。
//!
//! 语义与算法 `put_item` 的判定完全一致:对每件已放物品,
//! `支撑比 = Σ( y1 == y0 托底支撑物在 x/z 投影重叠面积 ) / (w×d)`;
//! 合法当且仅当 `支撑比 ≥ 1 − allowed_float_ratio`,或底面四角全部落实(兜底)。
//!
//! 覆盖:
//! 1. proptest 随机扫描(含随机允许悬空档位 0/0.25/0.5/0.75/1);
//! 2. best-load 种子数据真实场景(40HQ×2 + 20GP×3 + 474 件,纸箱 0.25、重件 0)。

use proptest::prelude::*;
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};

const EPS: f64 = 1e-9;
const FLOAT_RATIOS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// 单件合法性判定(与算法语义一致)。
fn is_legal(it: &Item, bin: &Bin) -> bool {
    let [w, _h, d] = it.dimension();
    let [x, y, z] = it.position;
    let bottom_area = w * d;
    if bottom_area <= EPS {
        return true; // 退化底面积,不判定
    }
    if y <= EPS {
        return true; // 落箱底:全支撑
    }

    let mut support = 0.0;
    for other in &bin.items {
        if std::ptr::eq(it, other) {
            continue;
        }
        let [ow, _, od] = other.dimension();
        let [ox, oy, oz] = other.position;
        let top = oy + other.dimension()[1];
        if (top - y).abs() > EPS {
            continue; // 顶面必须恰好托住底面
        }
        let x_ov = (x + w).min(ox + ow) - x.max(ox);
        let z_ov = (z + d).min(oz + od) - z.max(oz);
        if x_ov > EPS && z_ov > EPS {
            support += x_ov * z_ov;
        }
    }
    let ratio = support / bottom_area;
    if ratio + EPS >= 1.0 - it.allowed_float_ratio {
        return true;
    }

    // 四角兜底:底面四角各自落在某个 y1==y0 支撑矩形的 x/z 范围内。
    let in_rect = |cx: f64, cz: f64| -> bool {
        bin.items.iter().any(|other| {
            if std::ptr::eq(it, other) {
                return false;
            }
            let [ow, _, od] = other.dimension();
            let [ox, oy, oz] = other.position;
            let top = oy + other.dimension()[1];
            (top - y).abs() <= EPS
                && cx >= ox - EPS
                && cx <= ox + ow + EPS
                && cz >= oz - EPS
                && cz <= oz + od + EPS
        })
    };
    in_rect(x, z) && in_rect(x + w, z) && in_rect(x, z + d) && in_rect(x + w, z + d)
}

/// 检查全部箱子中每件已放物品的合法性。
fn check_all(p: &Packer, ctx: &str) {
    for bin in p.bins() {
        for it in &bin.items {
            if it.partno.starts_with("corner") {
                continue; // 角件是人工支撑件
            }
            assert!(
                is_legal(it, bin),
                "{ctx}: item {} in bin {} pos=({},{},{}) allowed={} violates support rule",
                it.partno,
                bin.partno,
                it.position[0],
                it.position[1],
                it.position[2],
                it.allowed_float_ratio
            );
        }
    }
}

proptest! {
    /// 随机扫描:任意合法输入,已放物品都必须满足支撑规则。
    #[test]
    fn no_floating_random_scan(
        bw in 1u32..=60,
        bh in 1u32..=60,
        bd in 1u32..=60,
        items in prop::collection::vec(
            // (w, h, d, weight, updown, 允许悬空档位)
            (1u32..=30, 1u32..=30, 1u32..=30, 1u32..=100, any::<bool>(), 0usize..5),
            1..=20
        ),
    ) {
        let mut packer = Packer::new();
        let mut bin = Bin::new("b", [bw as f64, bh as f64, bd as f64], 10_000.0);
        bin.put_type = 1;
        packer.add_bin(bin);
        for (i, (w, h, d, weight, updown, r)) in items.into_iter().enumerate() {
            packer.add_item(Item::new(
                format!("item{i}"),
                "test",
                ItemType::Cube,
                [w as f64, h as f64, d as f64],
                weight as f64,
                1,
                100,
                updown,
                "red",
                FLOAT_RATIOS[r],
            ));
        }
        packer.pack(&PackOptions {
            bigger_first: true,
            distribute_items: true,
            fix_point: true,
            check_stable: true,
            binding: vec![],
            number_of_decimals: 0,
        });
        check_all(&packer, "random");
    }
}

/// best-load 种子数据场景(docs/seed-test-data.sql):40HQ×2 + 20GP×3 + 474 件货。
fn seed_packer() -> Packer {
    let mut packer = Packer::new();
    for i in 0..2 {
        packer.add_bin(Bin::new(
            format!("bin-40hq-01#{i}"),
            [12032.0, 2698.0, 2352.0],
            26000.0,
        ));
    }
    for i in 0..3 {
        packer.add_bin(Bin::new(
            format!("bin-20gp-01#{i}"),
            [5898.0, 2393.0, 2352.0],
            21000.0,
        ));
    }
    let add = |packer: &mut Packer,
               id: &str,
               whd: [f64; 3],
               weight: f64,
               level: i32,
               updown: bool,
               allowed: f64,
               count: usize| {
        for i in 0..count {
            packer.add_item(Item::new(
                format!("{id}#{i}"),
                id,
                ItemType::Cube,
                whd,
                weight,
                level,
                30,
                updown,
                "#888888",
                allowed,
            ));
        }
    };
    add(
        &mut packer,
        "item-box-a",
        [600.0, 400.0, 500.0],
        12.5,
        0,
        true,
        0.25,
        150,
    );
    add(
        &mut packer,
        "item-carton-b",
        [280.0, 220.0, 180.0],
        3.2,
        0,
        true,
        0.25,
        300,
    );
    add(
        &mut packer,
        "item-machine-c",
        [2000.0, 1500.0, 1800.0],
        800.0,
        1,
        false,
        0.0,
        3,
    );
    add(
        &mut packer,
        "item-pallet-d",
        [1100.0, 1100.0, 1300.0],
        220.0,
        0,
        false,
        0.0,
        20,
    );
    add(
        &mut packer,
        "item-long-e",
        [13000.0, 3000.0, 3000.0],
        2000.0,
        2,
        false,
        0.0,
        1,
    );
    packer
}

/// 种子场景两种 options 下,已放物品全部合法,且未被新规则过度拒绝。
#[test]
fn seed_scenario_no_violations() {
    for (name, options) in [
        ("default", PackOptions::default()),
        (
            "bigger_first",
            PackOptions {
                bigger_first: true,
                ..PackOptions::default()
            },
        ),
    ] {
        let mut p = seed_packer();
        p.pack(&options);
        check_all(&p, &format!("seed/{name}"));

        let fitted: usize = p.bins.iter().map(|b| b.items.len()).sum();
        // 旧结果 473/474 件仍全装;新规则只校验支撑,不允许大幅回退。
        assert!(
            fitted >= 470,
            "seed/{name}: fitted {fitted}/474, support rule rejected too many"
        );
    }
}
