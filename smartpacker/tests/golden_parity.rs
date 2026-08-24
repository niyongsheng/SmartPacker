//! 黄金对齐集成测试：加载由 Python 源库（py3dbp）生成的基准 JSON，
//! 用 smartpacker 复现同一场景，并逐字段断言输出与 Python 完全一致。
//!
//! 基准数据由 `tools/gen_golden.py` 生成，提交入库；本测试不依赖 Python。

use serde::Deserialize;
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer};
use std::fs;
use std::path::PathBuf;

const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");
const TOL: f64 = 1e-9;

#[derive(Deserialize)]
struct Golden {
    #[allow(dead_code)]
    name: String,
    options: GoldenOptions,
    input: GoldenInput,
    expected: GoldenExpected,
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

#[derive(Deserialize)]
struct GoldenExpected {
    bins: Vec<GoldenBinExpected>,
    unfit_items: Vec<GoldenItemExpected>,
}

#[derive(Deserialize)]
struct GoldenBinExpected {
    partno: String,
    width: f64,
    height: f64,
    depth: f64,
    max_weight: f64,
    corner: f64,
    put_type: i32,
    gravity: Vec<f64>,
    items: Vec<GoldenItemExpected>,
    unfitted_items: Vec<GoldenItemExpected>,
}

#[derive(Deserialize)]
struct GoldenItemExpected {
    partno: String,
    name: String,
    #[serde(rename = "typeof")]
    type_of: String,
    width: f64,
    height: f64,
    depth: f64,
    weight: f64,
    color: String,
    rotation_type: u8,
    position: [f64; 3],
}

fn assert_f64(actual: f64, expected: f64, ctx: &str) {
    assert!(
        (actual - expected).abs() <= TOL,
        "{ctx}: expected {expected}, got {actual}"
    );
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
        let item = Item::new(
            it.partno.clone(),
            it.name.clone(),
            item_type_of(&it.type_of),
            it.whd,
            it.weight,
            it.level,
            it.loadbear,
            it.updown,
            it.color.clone(),
        );
        packer.add_item(item);
    }
    packer
}

fn run_scenario(path: &PathBuf) {
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

    assert_eq!(
        packer.bins.len(),
        golden.expected.bins.len(),
        "{name}: bin count"
    );

    for (bi, (actual_bin, expected_bin)) in packer
        .bins
        .iter()
        .zip(golden.expected.bins.iter())
        .enumerate()
    {
        let ctx = format!("{name}: bin[{bi}]");
        assert_eq!(actual_bin.partno, expected_bin.partno, "{ctx} partno");
        assert_f64(
            actual_bin.width,
            expected_bin.width,
            &format!("{ctx} width"),
        );
        assert_f64(
            actual_bin.height,
            expected_bin.height,
            &format!("{ctx} height"),
        );
        assert_f64(
            actual_bin.depth,
            expected_bin.depth,
            &format!("{ctx} depth"),
        );
        assert_f64(
            actual_bin.max_weight,
            expected_bin.max_weight,
            &format!("{ctx} max_weight"),
        );
        assert_f64(
            actual_bin.corner,
            expected_bin.corner,
            &format!("{ctx} corner"),
        );
        assert_eq!(actual_bin.put_type, expected_bin.put_type, "{ctx} put_type");

        assert_eq!(
            actual_bin.gravity.len(),
            expected_bin.gravity.len(),
            "{ctx} gravity len"
        );
        for (gi, (a, e)) in actual_bin
            .gravity
            .iter()
            .zip(expected_bin.gravity.iter())
            .enumerate()
        {
            assert_f64(*a, *e, &format!("{ctx} gravity[{gi}]"));
        }

        assert_eq!(
            actual_bin.items.len(),
            expected_bin.items.len(),
            "{ctx} items len"
        );
        for (ii, (a, e)) in actual_bin
            .items
            .iter()
            .zip(expected_bin.items.iter())
            .enumerate()
        {
            check_item(a, e, &format!("{ctx} item[{ii}]"));
        }

        assert_eq!(
            actual_bin.unfitted_items.len(),
            expected_bin.unfitted_items.len(),
            "{ctx} unfitted_items len"
        );
        for (ii, (a, e)) in actual_bin
            .unfitted_items
            .iter()
            .zip(expected_bin.unfitted_items.iter())
            .enumerate()
        {
            check_item(a, e, &format!("{ctx} unfitted[{ii}]"));
        }
    }

    assert_eq!(
        packer.unfit_items.len(),
        golden.expected.unfit_items.len(),
        "{name}: unfit_items len"
    );
    for (ii, (a, e)) in packer
        .unfit_items
        .iter()
        .zip(golden.expected.unfit_items.iter())
        .enumerate()
    {
        check_item(a, e, &format!("{name}: unfit[{ii}]"));
    }
}

fn check_item(actual: &Item, expected: &GoldenItemExpected, ctx: &str) {
    assert_eq!(actual.partno, expected.partno, "{ctx} partno");
    assert_eq!(actual.name, expected.name, "{ctx} name");
    let type_str = match actual.type_of {
        ItemType::Cube => "cube",
        ItemType::Cylinder => "cylinder",
    };
    assert_eq!(type_str, expected.type_of, "{ctx} typeof");
    assert_f64(actual.width, expected.width, &format!("{ctx} width"));
    assert_f64(actual.height, expected.height, &format!("{ctx} height"));
    assert_f64(actual.depth, expected.depth, &format!("{ctx} depth"));
    assert_f64(actual.weight, expected.weight, &format!("{ctx} weight"));
    assert_eq!(actual.color, expected.color, "{ctx} color");
    assert_eq!(
        actual.rotation_type, expected.rotation_type,
        "{ctx} rotation_type"
    );
    for (i, (a, e)) in actual
        .position
        .iter()
        .zip(expected.position.iter())
        .enumerate()
    {
        assert_f64(*a, *e, &format!("{ctx} position[{i}]"));
    }
}

#[test]
fn golden_parity_all_scenarios() {
    let files = golden_files();
    assert!(!files.is_empty(), "no golden files found");
    for f in &files {
        run_scenario(f);
    }
}
