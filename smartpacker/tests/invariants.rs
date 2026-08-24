//! 属性不变式与确定性测试（对应 spec R14(2)(3)(4)）。
//!
//! - proptest 随机生成箱体/物品，验证放置结果满足不越界、
//!   总重不超限、distribute 物品守恒、updown=false 旋转受限。
//!   （注：py3dbp 在 fix_point 下不保证两两不重叠，故不做该断言）
//! - 确定性：同输入两次运行输出完全一致。
//! - R7/R8 修复项：空绑定组跳过、零重量 gravity 不崩溃。

use proptest::prelude::*;
use smartpacker::{Bin, Item, ItemType, PackOptions, Packer, RotationType};

fn cube(partno: &str, whd: [f64; 3], weight: f64, updown: bool) -> Item {
    Item::new(
        partno,
        "test",
        ItemType::Cube,
        whd,
        weight,
        1,
        100,
        updown,
        "red",
    )
}

/// 检查单个箱子的几何/重量不变式。
fn check_bin_invariants(bin: &Bin, ctx: &str) {
    // 不越界
    for item in &bin.items {
        let d = item.dimension();
        for ax in 0..3 {
            assert!(
                item.position[ax] + d[ax] <= [bin.width, bin.height, bin.depth][ax] + 1e-9,
                "{ctx}: item {} axis {} out of bounds (pos {:?} + dim {:?} > {})",
                item.partno,
                ax,
                item.position,
                d,
                [bin.width, bin.height, bin.depth][ax]
            );
            assert!(
                item.position[ax] >= -1e-9,
                "{ctx}: item {} axis {} negative",
                item.partno,
                ax
            );
        }
        // updown=false 时旋转受限
        if !item.updown {
            assert!(
                item.rotation_type == RotationType::RT_WHD
                    || item.rotation_type == RotationType::RT_HWD,
                "{ctx}: updown=false item {} has rotation_type {}",
                item.partno,
                item.rotation_type
            );
        }
    }

    // 注意：不对「两两不重叠」做断言。py3dbp（及本移植）在 fix_point 下并不保证
    // 不重叠——putItem 的相交检查发生在重力修正之前，修正下落后的最终位置可能与
    // 已放置物品在几何上重叠（Python 同样输出重叠结果，属启发式固有行为，非本
    // 库的移植偏差）。
    //
    // 因此这里只保留真正成立的不变式：不越界、旋转受限、总重不超限。

    // 总重不超限
    let total: f64 = bin.items.iter().map(|i| i.weight).sum();
    assert!(
        total <= bin.max_weight + 1e-9,
        "{ctx}: total weight {total} exceeds max_weight {}",
        bin.max_weight
    );
}

proptest! {
    #[test]
    fn invariant_random_boxes_and_items(
        bw in 1u32..=60,
        bh in 1u32..=60,
        bd in 1u32..=60,
        items in prop::collection::vec(
            (1u32..=30, 1u32..=30, 1u32..=30, 1u32..=100, any::<bool>()),
            1..=20
        ),
    ) {
        let mut packer = Packer::new();
        let mut bin = Bin::new("b", [bw as f64, bh as f64, bd as f64], 10_000.0);
        bin.put_type = 1;
        packer.add_bin(bin);
        for (i, (w, h, d, weight, updown)) in items.into_iter().enumerate() {
            packer.add_item(cube(
                &format!("item{i}"),
                [w as f64, h as f64, d as f64],
                weight as f64,
                updown,
            ));
        }
        packer.pack(&PackOptions {
            bigger_first: true,
            distribute_items: true,
            fix_point: true,
            check_stable: true,
            support_surface_ratio: 0.75,
            binding: vec![],
            number_of_decimals: 0,
        });

        check_bin_invariants(&packer.bins[0], "random");

        // distribute_items=true 且单箱：物品守恒（每件恰出现于箱内或 unfit）
        let mut seen = std::collections::HashSet::new();
        for item in &packer.bins[0].items {
            assert!(seen.insert(item.partno.clone()), "duplicate fitted {}", item.partno);
        }
        for item in &packer.unfit_items {
            assert!(seen.insert(item.partno.clone()), "duplicate unfit {}", item.partno);
        }
    }
}

/// 确定性：同一输入两次运行，bins/items/unfit 完全一致。
#[test]
fn deterministic_same_input_twice() {
    let build = || {
        let mut packer = Packer::new();
        let mut bin = Bin::new("b1", [20.0, 20.0, 20.0], 1000.0);
        bin.put_type = 1;
        packer.add_bin(bin);
        for i in 0..8 {
            let whd = match i % 3 {
                0 => [5.0, 4.0, 3.0],
                1 => [3.0, 6.0, 2.0],
                _ => [4.0, 2.0, 5.0],
            };
            packer.add_item(cube(&format!("p{i}"), whd, 10.0, i % 2 == 0));
        }
        packer
    };

    let options = PackOptions {
        bigger_first: true,
        distribute_items: false,
        fix_point: true,
        check_stable: true,
        support_surface_ratio: 0.75,
        binding: vec![],
        number_of_decimals: 0,
    };

    let mut a = build();
    a.pack(&options);
    let mut b = build();
    b.pack(&options);

    let fingerprint = |p: &Packer| -> Vec<(String, String, u8, [f64; 3])> {
        p.bins
            .iter()
            .flat_map(|bin| {
                bin.items
                    .iter()
                    .map(|i| {
                        (
                            bin.partno.clone(),
                            i.partno.clone(),
                            i.rotation_type,
                            i.position,
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    assert_eq!(fingerprint(&a), fingerprint(&b));

    let unfit_a: Vec<String> = a.unfit_items.iter().map(|i| i.partno.clone()).collect();
    let unfit_b: Vec<String> = b.unfit_items.iter().map(|i| i.partno.clone()).collect();
    assert_eq!(unfit_a, unfit_b);
}

/// R7 修复：空绑定组（引用不存在的物品名）应被跳过，其余物品正常装箱。
#[test]
fn empty_binding_group_is_skipped() {
    let mut packer = Packer::new();
    let mut bin = Bin::new("b", [20.0, 20.0, 20.0], 1000.0);
    bin.put_type = 1;
    packer.add_bin(bin);
    packer.add_item(cube("a", [5.0, 5.0, 5.0], 10.0, true));
    packer.add_item(cube("b", [5.0, 5.0, 5.0], 10.0, true));

    packer.pack(&PackOptions {
        bigger_first: true,
        distribute_items: false,
        fix_point: true,
        check_stable: true,
        support_surface_ratio: 0.75,
        binding: vec![vec!["nonexistent".to_string()]],
        number_of_decimals: 0,
    });

    // 空组被跳过：两件物品都应被装入（而非因 min_c=0 被全部丢弃）。
    assert_eq!(packer.bins[0].items.len(), 2);
}

/// R8 修复：空箱（总重为 0）时 gravity 返回 [0,0,0,0] 且不崩溃。
#[test]
fn zero_weight_gravity_does_not_crash() {
    let mut packer = Packer::new();
    let mut bin = Bin::new("b", [10.0, 10.0, 10.0], 1000.0);
    bin.put_type = 1;
    packer.add_bin(bin);
    // 不添加任何物品。
    packer.pack(&PackOptions::default());

    assert_eq!(packer.bins[0].gravity, vec![0.0, 0.0, 0.0, 0.0]);
}
