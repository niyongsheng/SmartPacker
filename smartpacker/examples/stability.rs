//! 稳定性校验两条规则的演示(ex5/ex6 场景)。
//!
//! 规则一(ex5):底部支撑面积占比低于 `support_surface_ratio` 的物品被标记 unfit;
//! 规则二(ex6):底部四角任一悬空(无支撑)则移除该物品。
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
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        fix_point: true,
        check_stable: true,
        support_surface_ratio: 0.75,
        number_of_decimals: 0,
        ..PackOptions::default()
    });

    println!("[rule 1: support surface ratio={:.2}]", 0.75);
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
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        fix_point: true,
        check_stable: true,
        support_surface_ratio: 0.75,
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
