//! SmartPacker: 现代 3D 装箱打包库,由应用需求(best-load)驱动持续演进。
//!
//! 关键语义:
//! - 货物级稳定性:每件货物声明「允许悬空比例」`allowed_float_ratio`,
//!   判定规则为底面支撑面积占比 ≥ 1−allowed_float_ratio,或底面四角全部落实;
//! - 重力修正(fix_point)、绑定组、多箱分发(distribute)、圆柱体支持。
//! - 坐标系:x 右 / y 上 / z 前,原点位于容器底部左下角。

#![warn(missing_docs)]

pub mod auxiliary;
pub mod constants;
pub mod item;
pub mod packer;
#[cfg(feature = "plot")]
pub mod plot;

pub use auxiliary::{intersect, rect_intersect};
pub use constants::{Axis, ItemType, RotationType};
pub use item::Item;
pub use packer::{Bin, PackOptions, Packer};

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn packer_json_roundtrip_lossless() {
        let mut p = Packer::new();
        let mut b = Bin::new("box", [30.0, 10.0, 15.0], 99.0);
        b.put_type = 1;
        p.add_bin(b);
        for (i, whd) in [[9.0, 8.0, 7.0], [4.0, 25.0, 1.0], [2.0, 13.0, 5.0]]
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

        let json = serde_json::to_string(&p).expect("serialize packer");
        let back: Packer = serde_json::from_str(&json).expect("deserialize packer");
        let json2 = serde_json::to_string(&back).expect("re-serialize packer");
        assert_eq!(json, json2, "serde roundtrip must be lossless");
    }

    #[test]
    fn item_serializes_typeof_and_itemtype_lowercase() {
        let it = Item::new(
            "p",
            "n",
            ItemType::Cylinder,
            [1.0, 2.0, 3.0],
            1.0,
            1,
            100,
            true,
            "blue",
            0.25,
        );
        let v = serde_json::to_value(&it).expect("serialize item");
        assert_eq!(v["typeof"], "cylinder");
        assert_eq!(v["updown"], false);
    }
}
