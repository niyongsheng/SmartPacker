//! README 简单示例:一个 30×10×15 的箱子装入 5 个物品。
//!
//! 运行:`cargo run --example readme_simple`

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
            format!("test{}", i + 1),
            "test",
            ItemType::Cube,
            *whd,
            1.0,
            1,
            100,
            true,
            "red",
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        ..PackOptions::default()
    });

    for bin in &p.bins {
        println!(
            "bin {} ({}x{}x{}, max {}kg)",
            bin.partno, bin.width, bin.height, bin.depth, bin.max_weight
        );
        for it in &bin.items {
            println!(
                "  {} @ {:?} rotation={} whd={:?}",
                it.partno,
                it.position,
                it.rotation_type,
                it.dimension()
            );
        }
        for it in &bin.unfitted_items {
            println!("  UNFIT {}", it.partno);
        }
    }
}
