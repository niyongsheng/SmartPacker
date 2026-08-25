//! 多箱 + `distribute_items=true`:剩余物品自动分发给后续箱子。
//!
//! ex7 场景:5×5×5 与 3×3×5 两个箱子装 18 件货。
//! 运行:`cargo run --example multi_bin`

use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

fn main() {
    let mut p = Packer::new();
    p.add_bin(Bin::new("example7-Bin1", [5.0, 5.0, 5.0], 100.0));
    p.add_bin(Bin::new("example7-Bin2", [3.0, 3.0, 5.0], 100.0));

    // (partno, whd)
    const WHDS: [([f64; 3], [f64; 3]); 18] = [
        ([5.0, 4.0, 1.0], [0.0; 3]),
        ([1.0, 2.0, 4.0], [0.0; 3]),
        ([1.0, 2.0, 3.0], [0.0; 3]),
        ([1.0, 2.0, 2.0], [0.0; 3]),
        ([1.0, 2.0, 3.0], [0.0; 3]),
        ([1.0, 2.0, 4.0], [0.0; 3]),
        ([1.0, 2.0, 2.0], [0.0; 3]),
        ([1.0, 2.0, 3.0], [0.0; 3]),
        ([1.0, 2.0, 4.0], [0.0; 3]),
        ([1.0, 2.0, 3.0], [0.0; 3]),
        ([1.0, 2.0, 2.0], [0.0; 3]),
        ([5.0, 4.0, 1.0], [0.0; 3]),
        ([1.0, 1.0, 4.0], [0.0; 3]),
        ([1.0, 2.0, 1.0], [0.0; 3]),
        ([1.0, 2.0, 1.0], [0.0; 3]),
        ([1.0, 1.0, 4.0], [0.0; 3]),
        ([1.0, 1.0, 4.0], [0.0; 3]),
        ([5.0, 4.0, 2.0], [0.0; 3]),
    ];

    for (n, (whd, _)) in (1..).zip(WHDS.iter()) {
        p.add_item(Item::new(
            format!("Box-{n}"),
            "",
            ItemType::Cube,
            *whd,
            1.0,
            1,
            100,
            true,
            "olive",
            0.25,
        ));
    }

    p.pack(&PackOptions {
        bigger_first: true,
        distribute_items: true,
        ..PackOptions::default()
    });

    let total: usize = p.bins.iter().map(|b| b.items.len()).sum();
    println!("packed {total}/18 items across {} bins", p.bins.len());
    for bin in &p.bins {
        println!(
            "  bin {} ({}x{}x{}) -> {} items",
            bin.partno,
            bin.width,
            bin.height,
            bin.depth,
            bin.items.len()
        );
    }
    println!("globally unfit: {}", p.unfit_items.len());
}
