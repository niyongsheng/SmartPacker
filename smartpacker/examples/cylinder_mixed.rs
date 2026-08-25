//! 圆柱体与立方体混合装箱(ex1 场景精简版)。
//!
//! 展示 `ItemType::Cylinder` 与 `ItemType::Cube` 混装;圆柱体强制 `updown=false`。
//! 运行:`cargo run --example cylinder_mixed`

use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

/// (partno, type, whd, updown, color)
const ITEMS: [(&str, ItemType, [f64; 3], bool, &str); 8] = [
    ("powder1", ItemType::Cube, [2.0, 2.0, 4.0], true, "red"),
    ("powder2", ItemType::Cube, [2.0, 2.0, 4.0], true, "blue"),
    (
        "powder5",
        ItemType::Cylinder,
        [2.0, 2.0, 4.0],
        false,
        "lawngreen",
    ),
    (
        "powder8",
        ItemType::Cylinder,
        [4.0, 4.0, 2.0],
        false,
        "pink",
    ),
    (
        "powder9",
        ItemType::Cylinder,
        [4.0, 4.0, 2.0],
        false,
        "brown",
    ),
    ("powder10", ItemType::Cube, [4.0, 4.0, 2.0], true, "cyan"),
    (
        "powder12",
        ItemType::Cylinder,
        [4.0, 4.0, 2.0],
        false,
        "darkgreen",
    ),
    ("powder13", ItemType::Cube, [4.0, 4.0, 2.0], true, "orange"),
];

fn main() {
    let mut p = Packer::new();
    // 原场景 5.6875×10.75×15 量化后为 6×11×15;承重 70kg。
    p.add_bin(Bin::new("example1", [5.6875, 10.75, 15.0], 70.0));

    for (partno, ty, whd, _, _) in ITEMS {
        // 圆柱体由库强制 updown=false,Cube 透传 updown。
        p.add_item(Item::new(
            partno, "test", ty, whd, 1.0, 1, 100, true, "gray", 0.25,
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        distribute_items: false,
        ..PackOptions::default()
    });

    let bin = &p.bins[0];
    println!(
        "bin {} ({}x{}x{})",
        bin.partno, bin.width, bin.height, bin.depth
    );
    for it in &bin.items {
        let ty = match it.type_of {
            ItemType::Cube => "cube",
            ItemType::Cylinder => "cylinder",
        };
        println!(
            "  {:<8} type={:<8} updown={:<5} @ {:?} rot={}",
            it.partno, ty, it.updown, it.position, it.rotation_type
        );
    }
    println!("unfit at bin level: {}", bin.unfitted_items.len());
}
