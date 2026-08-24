//! 装箱效果图生成（`plot` feature）：把 tests/golden 下的真实场景打包后，用 plotters
//! 等距渲染成 PNG，供人工检查装箱效果。
//!
//! 运行:`cargo test --features plot --test plots -- --nocapture`
//! 输出目录:`<repo>/target/plots/`(已 gitignore),每个箱子一张 `<场景>__bin<n>.png`。

#![cfg(feature = "plot")]

use serde::Deserialize;
use smartpacker::plot::Painter;
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};
use std::fs;
use std::path::{Path, PathBuf};

const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");

#[derive(Deserialize)]
struct Golden {
    #[allow(dead_code)]
    name: String,
    options: GoldenOptions,
    input: GoldenInput,
}

#[derive(Deserialize)]
struct GoldenOptions {
    bigger_first: bool,
    distribute_items: bool,
    fix_point: bool,
    check_stable: bool,
    support_surface_ratio: f64,
    number_of_decimals: u32,
    #[serde(default)]
    binding: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct GoldenInput {
    bins: Vec<GoldenBinInput>,
    items: Vec<GoldenItemInput>,
}

#[derive(Deserialize)]
struct GoldenBinInput {
    partno: String,
    whd: [f64; 3],
    max_weight: f64,
    corner: f64,
    put_type: i32,
}

#[derive(Deserialize)]
struct GoldenItemInput {
    partno: String,
    name: String,
    #[serde(rename = "typeof")]
    type_of: String,
    whd: [f64; 3],
    weight: f64,
    level: i32,
    loadbear: i32,
    updown: bool,
    color: String,
}

fn item_type_of(s: &str) -> ItemType {
    match s {
        "cube" => ItemType::Cube,
        "cylinder" => ItemType::Cylinder,
        other => panic!("unknown typeof: {other}"),
    }
}

fn golden_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(GOLDEN_DIR)
        .expect("golden dir exists")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
        .collect();
    files.sort();
    files
}

fn build_packer(golden: &Golden) -> Packer {
    let mut packer = Packer::new();
    for b in &golden.input.bins {
        let mut bin = Bin::new(b.partno.clone(), b.whd, b.max_weight);
        bin.corner = b.corner;
        bin.put_type = b.put_type;
        packer.add_bin(bin);
    }
    for it in &golden.input.items {
        packer.add_item(Item::new(
            it.partno.clone(),
            it.name.clone(),
            item_type_of(&it.type_of),
            it.whd,
            it.weight,
            it.level,
            it.loadbear,
            it.updown,
            it.color.clone(),
        ));
    }
    packer
}

/// 解析输出目录:`<repo>/target/plots`(与 Cargo 共享 target,已 gitignore)。
fn plots_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("smartpacker is workspace member")
        .join("target")
        .join("plots")
}

fn render_scenario(path: &Path, out_dir: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(path).expect("read golden json");
    let golden: Golden = serde_json::from_str(&text).expect("parse golden json");
    let name = golden.name.clone();

    let mut packer = build_packer(&golden);
    let options = PackOptions {
        bigger_first: golden.options.bigger_first,
        distribute_items: golden.options.distribute_items,
        fix_point: golden.options.fix_point,
        check_stable: golden.options.check_stable,
        support_surface_ratio: golden.options.support_surface_ratio,
        binding: golden.options.binding.clone(),
        number_of_decimals: golden.options.number_of_decimals,
    };
    packer.pack(&options);

    let mut produced = Vec::new();
    for (bi, bin) in packer.bins.iter().enumerate() {
        let file = out_dir.join(format!("{name}__bin{bi}.png"));
        let title = format!("{name} · {} · {} items", bin.partno, bin.items.len());
        Painter::new(bin)
            .plot_box_and_items(&title, 0.8, true, 14, &file)
            .expect("render bin png");
        produced.push(file);
    }
    produced
}

/// 渲染全部黄金场景,输出到 target/plots,断言 PNG 存在且非空。
#[test]
fn render_golden_scenarios_to_png() {
    let files = golden_files();
    assert!(!files.is_empty(), "no golden files found");

    let out_dir = plots_dir();
    fs::create_dir_all(&out_dir).expect("create plots dir");

    let mut all = Vec::new();
    for f in &files {
        all.extend(render_scenario(f, &out_dir));
    }
    assert!(!all.is_empty(), "no png produced");

    for p in &all {
        let len = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        assert!(len > 0, "png must not be empty: {}", p.display());
    }

    println!(
        "plots written to {} ({} files)",
        out_dir.display(),
        all.len()
    );
    for p in &all {
        println!("  {}", p.display());
    }
}