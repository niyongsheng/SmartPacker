//! 数值与几何辅助函数。

use crate::constants::Axis;
use crate::item::Item;

/// 将浮点值向量化（银行家舍入，ROUND_HALF_EVEN），对齐
/// Python `Decimal(value).quantize(Decimal('1.000...'))` 的默认舍入语义。
///
/// `decimals` 控制保留的小数位数；`0` 表示取整到整数。
pub fn quantize(value: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    let scaled = value * factor;
    let rounded = round_half_even(scaled);
    rounded / factor
}

/// 银行家舍入（round half to even）。
fn round_half_even(x: f64) -> f64 {
    let floored = x.floor();
    let frac = x - floored;
    let round_up = if frac > 0.5 {
        true
    } else if frac < 0.5 {
        false
    } else {
        (floored as i64).rem_euclid(2) != 0
    };
    if round_up {
        floored + 1.0
    } else {
        floored
    }
}

/// 矩形相交判定（在 `x`、`y` 两个轴上比较投影），严格小于，即边界相贴不算相交。
///
/// 对齐 Python `rectIntersect`：以中心距的绝对值与两半宽/半高之和比较。
pub fn rect_intersect(item1: &Item, item2: &Item, x: usize, y: usize) -> bool {
    let d1 = item1.dimension();
    let d2 = item2.dimension();

    let cx1 = item1.position[x] + d1[x] / 2.0;
    let cy1 = item1.position[y] + d1[y] / 2.0;
    let cx2 = item2.position[x] + d2[x] / 2.0;
    let cy2 = item2.position[y] + d2[y] / 2.0;

    let ix = (cx1 - cx2).abs();
    let iy = (cy1 - cy2).abs();

    ix < (d1[x] + d2[x]) / 2.0 && iy < (d1[y] + d2[y]) / 2.0
}

/// 三维相交判定：在（宽,高）、（高,深）、（宽,深）三对轴上分别做矩形相交判定。
///
/// 对齐 Python `intersect`。
pub fn intersect(item1: &Item, item2: &Item) -> bool {
    rect_intersect(item1, item2, Axis::WIDTH, Axis::HEIGHT)
        && rect_intersect(item1, item2, Axis::HEIGHT, Axis::DEPTH)
        && rect_intersect(item1, item2, Axis::WIDTH, Axis::DEPTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::ItemType;

    fn item_at(pos: [f64; 3], whd: [f64; 3]) -> Item {
        let mut it = Item::new(
            "p",
            "n",
            ItemType::Cube,
            whd,
            1.0,
            1,
            100,
            true,
            "red",
            0.25,
        );
        it.position = pos;
        it
    }

    #[test]
    fn quantize_rounds_half_to_even() {
        assert_eq!(quantize(2.5, 0), 2.0);
        assert_eq!(quantize(3.5, 0), 4.0);
        assert_eq!(quantize(-2.5, 0), -2.0);
        assert_eq!(quantize(-3.5, 0), -4.0);
        assert_eq!(quantize(2.4, 0), 2.0);
        assert_eq!(quantize(2.6, 0), 3.0);
        assert_eq!(quantize(85.12, 1), 85.1);
        assert_eq!(quantize(589.8, 0), 590.0);
        assert_eq!(quantize(259.1, 0), 259.0);
    }

    #[test]
    fn intersect_shares_face_is_not_overlap() {
        // 两个 2x2x2 立方体在 x 轴贴面：item1 占 x∈[0,2)，item2 占 x∈[2,4)
        let a = item_at([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = item_at([2.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        assert!(!intersect(&a, &b));
    }

    #[test]
    fn intersect_overlap_is_detected() {
        let a = item_at([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = item_at([1.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        assert!(intersect(&a, &b));
    }

    #[test]
    fn intersect_disjoint_is_false() {
        let a = item_at([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = item_at([5.0, 5.0, 5.0], [1.0, 1.0, 1.0]);
        assert!(!intersect(&a, &b));
    }
}
