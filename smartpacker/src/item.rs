//! 物品（Item）类型，对应 Python `py3dbp/main.py` 的 `Item` 类。

use crate::auxiliary::quantize;
use crate::constants::{ItemType, RotationType};
use std::fmt;

/// 三维装箱中的一件物品。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Item {
    /// 唯一编号（partno / PN）。
    pub partno: String,
    /// 物品类型名称。
    pub name: String,
    /// 物品形态（立方体或圆柱体）。
    #[cfg_attr(feature = "serde", serde(rename = "typeof"))]
    pub type_of: ItemType,
    /// 宽（width）。
    pub width: f64,
    /// 高（height）。
    pub height: f64,
    /// 深（depth）。
    pub depth: f64,
    /// 重量。
    pub weight: f64,
    /// 装箱优先级 level（越小优先级越高）。
    pub level: i32,
    /// 承重能力 loadbear（越大优先级越高）。
    pub loadbear: i32,
    /// 是否允许倒放。
    pub updown: bool,
    /// 显示颜色。
    pub color: String,
    /// 旋转类型（0..5，见 [`RotationType`]）。
    pub rotation_type: u8,
    /// 当前位置（x, y, z）。
    pub position: [f64; 3],
    /// 数值量化保留的小数位数。
    pub number_of_decimals: u32,
}

impl Item {
    /// 构造一件物品。
    ///
    /// 当 `type_of` 不是 [`ItemType::Cube`] 时，`updown` 会被强制为 `false`（对齐 Python 行为）。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        partno: impl Into<String>,
        name: impl Into<String>,
        type_of: ItemType,
        whd: [f64; 3],
        weight: f64,
        level: i32,
        loadbear: i32,
        updown: bool,
        color: impl Into<String>,
    ) -> Self {
        let updown = if type_of == ItemType::Cube {
            updown
        } else {
            false
        };
        Item {
            partno: partno.into(),
            name: name.into(),
            type_of,
            width: whd[0],
            height: whd[1],
            depth: whd[2],
            weight,
            level,
            loadbear,
            updown,
            color: color.into(),
            rotation_type: RotationType::RT_WHD,
            position: [0.0, 0.0, 0.0],
            number_of_decimals: 0,
        }
    }

    /// 对宽、高、深、重量做量化（ROUND_HALF_EVEN），并记录小数位数。
    pub fn format_numbers(&mut self, number_of_decimals: u32) {
        self.width = quantize(self.width, number_of_decimals);
        self.height = quantize(self.height, number_of_decimals);
        self.depth = quantize(self.depth, number_of_decimals);
        self.weight = quantize(self.weight, number_of_decimals);
        self.number_of_decimals = number_of_decimals;
    }

    /// 体积。
    pub fn volume(&self) -> f64 {
        quantize(
            self.width * self.height * self.depth,
            self.number_of_decimals,
        )
    }

    /// 最大底面积：可倒放时取最大两维的乘积，否则取宽×高。
    pub fn max_area(&self) -> f64 {
        let a = if self.updown {
            let mut v = [self.width, self.height, self.depth];
            v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            v
        } else {
            [self.width, self.height, self.depth]
        };
        quantize(a[0] * a[1], self.number_of_decimals)
    }

    /// 返回当前旋转类型下的尺寸 `[x, y, z]`（镜像 Python `getDimension`）。
    pub fn dimension(&self) -> [f64; 3] {
        let (w, h, d) = (self.width, self.height, self.depth);
        match self.rotation_type {
            RotationType::RT_WHD => [w, h, d],
            RotationType::RT_HWD => [h, w, d],
            RotationType::RT_HDW => [h, d, w],
            RotationType::RT_DHW => [d, h, w],
            RotationType::RT_DWH => [d, w, h],
            RotationType::RT_WDH => [w, d, h],
            _ => [w, h, d],
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({}x{}x{}, weight: {}) pos({},{},{}) rt({}) vol({})",
            self.partno,
            self.width,
            self.height,
            self.depth,
            self.weight,
            self.position[0],
            self.position[1],
            self.position[2],
            self.rotation_type,
            self.volume()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(whd: [f64; 3], updown: bool) -> Item {
        Item::new("p", "n", ItemType::Cube, whd, 1.0, 1, 100, updown, "red")
    }

    #[test]
    fn cylinder_forces_updown_false() {
        let it = Item::new(
            "p",
            "n",
            ItemType::Cylinder,
            [1.0, 2.0, 3.0],
            1.0,
            1,
            100,
            true,
            "red",
        );
        assert!(!it.updown);
    }

    #[test]
    fn display_shows_identity_and_geometry() {
        let it = cube([2.0, 3.0, 4.0], true);
        let s = it.to_string();
        // 格式: `<partno>(<w>x<h>x<d>, weight: <w>) pos(<x>,<y>,<z>) rt(<rt>) vol(<vol>)`
        assert_eq!(s, "p(2x3x4, weight: 1) pos(0,0,0) rt(0) vol(24)");
    }

    #[test]
    fn dimension_permutes_by_rotation() {
        let it = cube([1.0, 2.0, 3.0], true);
        assert_eq!(it.dimension(), [1.0, 2.0, 3.0]);
        let mut it = it;
        it.rotation_type = RotationType::RT_HWD;
        assert_eq!(it.dimension(), [2.0, 1.0, 3.0]);
        it.rotation_type = RotationType::RT_HDW;
        assert_eq!(it.dimension(), [2.0, 3.0, 1.0]);
        it.rotation_type = RotationType::RT_DHW;
        assert_eq!(it.dimension(), [3.0, 2.0, 1.0]);
        it.rotation_type = RotationType::RT_DWH;
        assert_eq!(it.dimension(), [3.0, 1.0, 2.0]);
        it.rotation_type = RotationType::RT_WDH;
        assert_eq!(it.dimension(), [1.0, 3.0, 2.0]);
    }

    #[test]
    fn volume_and_max_area() {
        let it = cube([2.0, 3.0, 4.0], true);
        assert_eq!(it.volume(), 24.0);
        // 可倒放：取最大两维 4*3
        assert_eq!(it.max_area(), 12.0);
        let it = cube([2.0, 3.0, 4.0], false);
        // 不可倒放：宽×高 2*3
        assert_eq!(it.max_area(), 6.0);
    }

    #[test]
    fn format_numbers_quantizes() {
        let mut it = Item::new(
            "p",
            "n",
            ItemType::Cube,
            [589.8, 243.8, 259.1],
            85.12,
            1,
            100,
            true,
            "red",
        );
        it.format_numbers(0);
        assert_eq!(it.width, 590.0);
        assert_eq!(it.height, 244.0);
        assert_eq!(it.depth, 259.0);
        assert_eq!(it.weight, 85.0);
    }
}
