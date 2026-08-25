//! 绑定组(binding):不同名称的物品轮询交错摆放。
//!
//! apple 与 orange 绑定为一组,a1↔o1、a2↔o2 轮询交替;组内长序盈余
//! (若某名称数量多于组内其它名称)会进入 unfit。
//! 运行:`cargo run --example binding`

use smartpacker::constants::ItemType;
use smartpacker::item::Item;
use smartpacker::packer::{Bin, PackOptions, Packer};

fn main() {
    let mut p = Packer::new();
    let mut bin = Bin::new("binding", [30.0, 20.0, 15.0], 1000.0);
    bin.put_type = 1;
    p.add_bin(bin);

    // apple 2 件、orange 2 件、free 1 件(不绑定)。
    const ITEMS: [(&str, &str, [f64; 3], &str); 5] = [
        ("a1", "apple", [5.0, 5.0, 5.0], "red"),
        ("a2", "apple", [5.0, 5.0, 5.0], "red"),
        ("o1", "orange", [4.0, 4.0, 4.0], "blue"),
        ("o2", "orange", [4.0, 4.0, 4.0], "blue"),
        ("free1", "free", [6.0, 6.0, 6.0], "green"),
    ];
    for (partno, name, whd, color) in ITEMS {
        p.add_item(Item::new(
            partno,
            name,
            ItemType::Cube,
            whd,
            1.0,
            1,
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
        binding: vec![vec!["apple".into(), "orange".into()]],
        ..PackOptions::default()
    });

    let bin = &p.bins[0];
    println!(
        "bin {} ({}x{}x{}, put_type={})",
        bin.partno, bin.width, bin.height, bin.depth, bin.put_type
    );
    for it in &bin.items {
        println!(
            "  {:>6} (name={:<6}) @ {:?} rot={}",
            it.partno, it.name, it.position, it.rotation_type
        );
    }
    for it in &bin.unfitted_items {
        println!("  UNFIT {}({})", it.partno, it.name);
    }
}
