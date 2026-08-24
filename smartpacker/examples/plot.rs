//! 可视化示例:将 readme_simple 场景渲染成 PNG。
//!
//! `plot` 是可选 feature(依赖 plotters),需显式开启:
//! `cargo run --example plot --features plot`
//! 输出文件为当前目录下的 `example_plot.png`。

#[cfg(not(feature = "plot"))]
fn main() {
    println!("this example needs the `plot` feature: cargo run --example plot --features plot");
}

#[cfg(feature = "plot")]
fn main() {
    use smartpacker::constants::ItemType;
    use smartpacker::item::Item;
    use smartpacker::packer::{Bin, PackOptions, Packer};
    use smartpacker::plot::Painter;

    let mut p = Packer::new();
    p.add_bin(Bin::new("example", [30.0, 10.0, 15.0], 99.0));
    for (i, whd) in [
        [9.0, 8.0, 7.0],
        [4.0, 25.0, 1.0],
        [2.0, 13.0, 5.0],
        [7.0, 5.0, 4.0],
        [10.0, 5.0, 2.0],
    ]
    .iter()
    .copied()
    .enumerate()
    {
        p.add_item(Item::new(
            format!("test{}", i + 1),
            "test",
            ItemType::Cube,
            whd,
            1.0,
            1,
            100,
            true,
            "red",
        ));
    }
    p.pack(&PackOptions::default());

    let out = "example_plot.png";
    let bin = &p.bins[0];
    Painter::new(bin)
        .plot_box_and_items("example", 0.8, true, 14, out)
        .expect("render PNG");
    println!("wrote {out} with {} items", bin.items.len());
}
