//! 装箱效果图生成(`plot` feature):自建场景打包后,用 plotters 等距渲染成 PNG,
//! 供人工检查装箱效果(不再依赖已删除的黄金用例)。
//!
//! 运行:`cargo test --features plot --test plots -- --nocapture`
//! 输出目录:`<repo>/target/plots/`(已 gitignore),每个箱子一张 `<场景>__bin<n>.png`。

#![cfg(feature = "plot")]

use smartpacker::plot::Painter;
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};
use std::fs;
use std::path::{Path, PathBuf};

/// 解析输出目录:`<repo>/target/plots`(与 Cargo 共享 target,已 gitignore)。
fn plots_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("smartpacker is workspace member")
        .join("target")
        .join("plots")
}

fn render_bins(p: &Packer, name: &str, out_dir: &Path, produced: &mut Vec<PathBuf>) {
    for (bi, bin) in p.bins.iter().enumerate() {
        let file = out_dir.join(format!("{name}__bin{bi}.png"));
        let title = format!("{name} · {} · {} items", bin.partno, bin.items.len());
        Painter::new(bin)
            .plot_box_and_items(&title, 0.8, true, 14, &file)
            .expect("render bin png");
        produced.push(file);
    }
}

/// 渲染自建场景,输出到 target/plots,断言 PNG 存在且非空。
#[test]
fn render_scenarios_to_png() {
    let out_dir = plots_dir();
    fs::create_dir_all(&out_dir).expect("create plots dir");
    let mut produced: Vec<PathBuf> = Vec::new();

    // 场景 1:readme_simple(30×10×15 装 5 件)
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
            0.25,
        ));
    }
    p.pack(&PackOptions {
        bigger_first: true,
        ..PackOptions::default()
    });
    render_bins(&p, "readme_simple", &out_dir, &mut produced);

    // 场景 2:圆柱混合(5.6875×10.75×15,量化后 6×11×15)
    let mut p = Packer::new();
    p.add_bin(Bin::new("example1", [5.6875, 10.75, 15.0], 70.0));
    for (partno, ty) in [
        ("powder1", ItemType::Cube),
        ("powder2", ItemType::Cube),
        ("powder5", ItemType::Cylinder),
        ("powder8", ItemType::Cylinder),
        ("powder9", ItemType::Cylinder),
        ("powder10", ItemType::Cube),
        ("powder12", ItemType::Cylinder),
        ("powder13", ItemType::Cube),
    ] {
        p.add_item(Item::new(
            partno,
            "test",
            ty,
            [2.0, 2.0, 4.0],
            1.0,
            1,
            100,
            ty == ItemType::Cube,
            "gray",
            0.25,
        ));
    }
    p.pack(&PackOptions {
        bigger_first: true,
        distribute_items: false,
        ..PackOptions::default()
    });
    render_bins(&p, "cylinder_mixed", &out_dir, &mut produced);

    // 场景 3:多箱分发(5×5×5 + 3×3×5 装 18 件)
    let mut p = Packer::new();
    p.add_bin(Bin::new("example7-Bin1", [5.0, 5.0, 5.0], 100.0));
    p.add_bin(Bin::new("example7-Bin2", [3.0, 3.0, 5.0], 100.0));
    for (n, whd) in [
        [5.0, 4.0, 1.0],
        [1.0, 2.0, 4.0],
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 2.0],
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 4.0],
        [1.0, 2.0, 2.0],
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 4.0],
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 2.0],
        [5.0, 4.0, 1.0],
        [1.0, 1.0, 4.0],
        [1.0, 2.0, 1.0],
        [1.0, 2.0, 1.0],
        [1.0, 1.0, 4.0],
        [1.0, 1.0, 4.0],
        [5.0, 4.0, 2.0],
    ]
    .iter()
    .enumerate()
    {
        p.add_item(Item::new(
            format!("Box-{}", n + 1),
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
    render_bins(&p, "multi_bin", &out_dir, &mut produced);

    assert!(!produced.is_empty(), "no png produced");
    for p in &produced {
        let len = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        assert!(len > 0, "png must not be empty: {}", p.display());
    }

    println!(
        "plots written to {} ({} files)",
        out_dir.display(),
        produced.len()
    );
    for p in &produced {
        println!("  {}", p.display());
    }
}
