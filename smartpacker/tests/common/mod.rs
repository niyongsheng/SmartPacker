//! 门禁测试共享工具：支撑规则判定与种子场景。
//!
//! 支撑判定是「独立 oracle」——有意不调用算法内部的 `bottom_support`，
//! 而是按规则文档独立实现一遍，防止算法与校验器同错；两个测试文件
//! 与 floating_check 示例共用同一份实现，规则或容差只在一处维护。

use smartpacker::constants::ItemType;
use smartpacker::{Bin, Item, Packer};

/// 几何比较容差。
pub const EPS: f64 = 1e-9;
/// 随机扫描的允许悬空档位。
pub const FLOAT_RATIOS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

/// 单件支撑统计：`(支撑面积占比, 底面四角是否全部落实)`。
///
/// 规则（与算法 put_item 的判定一致）：支撑比 = Σ( y1 == y0 托底的支撑物在
/// x/z 投影重叠面积 ) / (w×d)；合法当且仅当 支撑比 ≥ 1−allowed_float_ratio，
/// 或底面四角全部落实（兜底）。落箱底（y≈0）与零底面积视为全支撑。
pub fn support_stats(it: &Item, bin: &Bin) -> (f64, bool) {
    let [w, _h, d] = it.dimension();
    let [x, y, z] = it.position;
    let bottom_area = w * d;
    if bottom_area <= EPS {
        return (1.0, true); // 退化底面积，不判定
    }
    if y <= EPS {
        return (1.0, true); // 落箱底：全支撑
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

    // 四角兜底：底面四角各自落在某个 y1==y0 支撑矩形的 x/z 范围内。
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
    let corners_ok =
        in_rect(x, z) && in_rect(x + w, z) && in_rect(x, z + d) && in_rect(x + w, z + d);
    (ratio, corners_ok)
}

/// 单件合法性判定（支撑比达标或四角兜底）。
/// 仅供测试文件使用；floating_check 示例直接组合 `support_stats` 的结果避免重复遍历。
#[allow(dead_code)]
pub fn is_legal(it: &Item, bin: &Bin) -> bool {
    let (ratio, corners_ok) = support_stats(it, bin);
    ratio + EPS >= 1.0 - it.allowed_float_ratio || corners_ok
}

/// best-load 种子数据场景（docs/seed-test-data.sql）：40HQ×2 + 20GP×3 + 474 件货。
/// allowed_float_ratio 与计划一致：纸箱 A/B 0.25，重型设备/托盘/长件 0（必须稳妥支撑）。
/// 仅供门禁测试与 floating_check 示例使用（并非每个编译单元都用到）。
#[allow(dead_code)]
pub fn seed_packer() -> Packer {
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
