//! 底部支撑检查两条规则的演示。
//!
//! 规则一:底面支撑面积占比低于 `1 - 允许悬空比例` 的物品被拒绝;
//! 规则二:底面四角任一悬空(无支撑)时,四角全部落实可兜底放行。
//! 运行:`cargo run --example stability`

use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

fn main() {
    demo_rule1_support_surface();
    demo_rule2_four_corner_support();
}

/// 规则一:支撑面比检查。
fn demo_rule1_support_surface() {
    let mut p = Packer::new();
    p.add_bin(Bin::new("example5", [5.0, 4.0, 3.0], 100.0));

    const ITEMS: [(&str, [f64; 3], i32, &str); 3] = [
        ("Box-3", [2.0, 5.0, 2.0], 1, "pink"),
        ("Box-3", [2.0, 3.0, 2.0], 2, "pink"),
        ("Box-4", [5.0, 4.0, 1.0], 3, "brown"),
    ];
    for (partno, whd, level, color) in ITEMS {
        p.add_item(Item::new(
            partno,
            "test",
            ItemType::Cube,
            whd,
            1.0,
            level,
            100,
            true,
            color,
            0.25,
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        fix_point: true,
        check_stable: true,
        number_of_decimals: 0,
        ..PackOptions::default()
    });

    println!("[rule 1: support ratio >= 1 - allowed_float (0.25)]");
    let bin = &p.bins[0];
    for it in &bin.items {
        println!(
            "  fitted   {:>6} @ {:?} rot={}",
            it.partno, it.position, it.rotation_type
        );
    }
    for it in &bin.unfitted_items {
        println!(
            "  unstable {:>6} @ {:?} rot={}",
            it.partno, it.position, it.rotation_type
        );
    }
}

/// 规则二:四角支撑检查。
fn demo_rule2_four_corner_support() {
    let mut p = Packer::new();
    p.add_bin(Bin::new("example6", [5.0, 4.0, 7.0], 100.0));

    const WHDS: [[f64; 3]; 9] = [
        [5.0, 4.0, 1.0],
        [1.0, 1.0, 4.0],
        [3.0, 4.0, 2.0],
        [1.0, 1.0, 4.0],
        [1.0, 2.0, 1.0],
        [1.0, 2.0, 1.0],
        [1.0, 1.0, 4.0],
        [1.0, 1.0, 4.0],
        [5.0, 4.0, 2.0],
    ];
    const COLORS: [&str; 9] = [
        "yellow", "olive", "pink", "olive", "pink", "pink", "olive", "olive", "brown",
    ];
    for (i, whd) in WHDS.iter().enumerate() {
        p.add_item(Item::new(
            format!("Box-{}", i + 1),
            "test",
            ItemType::Cube,
            *whd,
            1.0,
            (i + 1) as i32,
            100,
            true,
            COLORS[i],
            0.25,
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        fix_point: true,
        check_stable: true,
        number_of_decimals: 0,
        ..PackOptions::default()
    });

    println!("\n[rule 2: four-corner support]");
    let bin = &p.bins[0];
    for it in &bin.items {
        println!(
            "  fitted   {:>6} @ {:?} rot={}",
            it.partno, it.position, it.rotation_type
        );
    }
    for it in &bin.unfitted_items {
        println!(
            "  unstable {:>6} @ {:?} rot={}",
            it.partno, it.position, it.rotation_type
        );
    }
}
