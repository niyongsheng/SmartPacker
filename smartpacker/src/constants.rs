//! 常量定义：旋转类型、轴向、物品类型。

/// 旋转类型（作为原始索引常量，非穷举枚举）。
///
/// 每种旋转类型对应不同的 WHD 置换方案，见 [`crate::Item::dimension`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RotationType;

impl RotationType {
    /// 原样放置：width, height, depth。
    pub const RT_WHD: u8 = 0;
    /// height, width, depth。
    pub const RT_HWD: u8 = 1;
    /// height, depth, width。
    pub const RT_HDW: u8 = 2;
    /// depth, height, width。
    pub const RT_DHW: u8 = 3;
    /// depth, width, height。
    pub const RT_DWH: u8 = 4;
    /// width, depth, height。
    pub const RT_WDH: u8 = 5;

    /// 可倒放物品允许的全部 6 种旋转。
    pub const ALL: [u8; 6] = [
        Self::RT_WHD,
        Self::RT_HWD,
        Self::RT_HDW,
        Self::RT_DHW,
        Self::RT_DWH,
        Self::RT_WDH,
    ];

    /// 禁止倒放物品仅允许的 2 种旋转。
    pub const NOT_UPDOWN: [u8; 2] = [Self::RT_WHD, Self::RT_HWD];
}

/// 轴向，取值与原库 `Axis` 类一致。
pub struct Axis;

impl Axis {
    /// 宽度方向（x 轴）索引。
    pub const WIDTH: usize = 0;
    /// 高度方向（y 轴）索引。
    pub const HEIGHT: usize = 1;
    /// 深度方向（z 轴）索引。
    pub const DEPTH: usize = 2;
}

/// 物品类型：立方体或圆柱体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ItemType {
    /// 立方体。
    Cube,
    /// 圆柱体（不可倒放，`updown` 会被强制为 `false`）。
    Cylinder,
}
