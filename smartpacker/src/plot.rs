//! 可视化（可选 `plot` feature）：基于 plotters 的等距投影 3D 渲染。
//!
//! 对齐 Python `Painter` 的 API 意图（`Painter::new(bin)` +
//! `plot_box_and_items(...)`），输出 PNG；不追求与 matplotlib 像素级一致。
//!
//! - 箱体：黑色线框。
//! - 物品：立方体绘制三个可见面（顶 + 两个侧面），圆柱体绘制上/下椭圆与侧面示意。
//! - 可选标注物品 `partno`。

use crate::constants::ItemType;
use crate::packer::Bin;
use std::io;
use std::path::Path;

use plotters::coord::cartesian::Cartesian2d;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

/// 本模块使用的 2D 坐标图类型（使用时由函数签名提供 `DB: DrawingBackend`）。
type Chart<'a, DB> = ChartContext<'a, DB, Cartesian2d<RangedCoordf64, RangedCoordf64>>;

/// `cos(30°)`。
const COS30: f64 = 0.866_025_403_784_438_6;
/// `sin(30°)`。
const SIN30: f64 = 0.5;

/// 等距投影：3D 坐标 → 2D 世界坐标（屏幕向右为 +x，向上为 +高度）。
fn project(p: [f64; 3]) -> (f64, f64) {
    let (x, y, z) = (p[0], p[1], p[2]);
    ((x - z) * COS30, y - (x + z) * SIN30)
}

/// 将 plotters 的绘制错误转换为 `io::Error`。
fn io_err(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e.to_string())
}

/// 颜色名称 → RGB。支持 `#RRGGBB` 十六进制与常见命名色；未知名回退到确定性散列。
fn rgb_from_name(name: &str) -> RGBColor {
    if let Some(hex) = name.strip_prefix('#') {
        if hex.len() >= 6 {
            let r = u8::from_str_radix(&hex[0..2], 16);
            let g = u8::from_str_radix(&hex[2..4], 16);
            let b = u8::from_str_radix(&hex[4..6], 16);
            if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                return RGBColor(r, g, b);
            }
        }
    }
    let lower = name.to_ascii_lowercase();
    let rgb = match lower.as_str() {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "blue" => (0, 0, 255),
        "green" => (0, 128, 0),
        "lime" | "lawngreen" => (124, 252, 0),
        "yellow" => (255, 255, 0),
        "orange" => (255, 165, 0),
        "purple" => (128, 0, 128),
        "brown" => (165, 42, 42),
        "gray" | "grey" => (128, 128, 128),
        "cyan" | "aqua" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "pink" => (255, 192, 203),
        "navy" => (0, 0, 128),
        "teal" => (0, 128, 128),
        "olive" => (128, 128, 0),
        "maroon" => (128, 0, 0),
        "silver" => (192, 192, 192),
        "gold" => (255, 215, 0),
        _ => return hash_color(name),
    };
    RGBColor(rgb.0, rgb.1, rgb.2)
}

/// 未知颜色名的确定性散列（保证同一名称颜色稳定）。
fn hash_color(name: &str) -> RGBColor {
    let mut h: u32 = 5381;
    for b in name.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    RGBColor((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

/// 将颜色向白色混合以模拟透明度（`alpha ∈ [0,1]`，越大越不透明）。
fn with_alpha(c: RGBColor, alpha: f64) -> RGBColor {
    let a = alpha.clamp(0.0, 1.0);
    let blend = |v: u8| ((v as f64 * a + 255.0 * (1.0 - a)).round() as u8).clamp(0, 255);
    RGBColor(blend(c.0), blend(c.1), blend(c.2))
}

/// 求一组投影点的包围盒与等宽高比坐标范围（保持 canvas 宽高比，避免畸变）。
fn equal_aspect(
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    canvas: (u32, u32),
) -> (std::ops::Range<f64>, std::ops::Range<f64>) {
    let mut w = (max_x - min_x).max(1e-6);
    let mut h = (max_y - min_y).max(1e-6);
    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    let aspect = canvas.0 as f64 / canvas.1 as f64;
    if w / h < aspect {
        w = h * aspect;
    } else {
        h = w / aspect;
    }
    let pad = 1.12; // 额外留白
    w *= pad;
    h *= pad;
    (cx - w / 2.0..cx + w / 2.0, cy - h / 2.0..cy + h / 2.0)
}

/// 三维装箱可视化器（对应 Python `Painter`，一次绑定一个箱子）。
pub struct Painter<'a> {
    bin: &'a Bin,
}

impl<'a> Painter<'a> {
    /// 绑定要渲染的箱子。
    pub fn new(bin: &'a Bin) -> Self {
        Painter { bin }
    }

    /// 渲染箱子及其物品到 PNG 文件。
    ///
    /// `alpha` 控制物品填充不透明度（`0.0` 透明 ~ `1.0` 不透明）；`write_num` 决定
    /// 是否在物品上标注 `partno`；`fontsize` 为标注字号。
    pub fn plot_box_and_items<P: AsRef<Path>>(
        &self,
        title: &str,
        alpha: f64,
        write_num: bool,
        fontsize: u32,
        path: P,
    ) -> io::Result<()> {
        let canvas = (1024u32, 768u32);
        let root = BitMapBackend::new(path.as_ref(), canvas).into_drawing_area();
        root.fill(&WHITE).map_err(io_err)?;

        let (min_x, max_x, min_y, max_y) = self.bounds();
        let (xr, yr) = equal_aspect(min_x, max_x, min_y, max_y, canvas);

        {
            let mut chart = ChartBuilder::on(&root)
                .margin(25)
                .caption(title, ("sans-serif", 18).into_font())
                .build_cartesian_2d(xr, yr)
                .map_err(io_err)?;
            self.draw_bin(&mut chart)?;
            self.draw_items(&mut chart, alpha, write_num, fontsize)?;
        }

        root.present().map_err(io_err)
    }

    /// 收集箱子与所有物品的角点并求投影包围盒。
    fn bounds(&self) -> (f64, f64, f64, f64) {
        let mut pts: Vec<[f64; 3]> = Vec::new();
        let (w, h, d) = (self.bin.width, self.bin.height, self.bin.depth);
        for &(x, y, z) in &[
            (0.0, 0.0, 0.0),
            (w, 0.0, 0.0),
            (w, h, 0.0),
            (0.0, h, 0.0),
            (0.0, 0.0, d),
            (w, 0.0, d),
            (w, h, d),
            (0.0, h, d),
        ] {
            pts.push([x, y, z]);
        }
        for item in &self.bin.items {
            let (x, y, z) = (item.position[0], item.position[1], item.position[2]);
            let dim = item.dimension();
            let (iw, ih, id) = (dim[0], dim[1], dim[2]);
            for &(dx, dy, dz) in &[
                (0.0, 0.0, 0.0),
                (iw, 0.0, 0.0),
                (iw, ih, 0.0),
                (0.0, ih, 0.0),
                (0.0, 0.0, id),
                (iw, 0.0, id),
                (iw, ih, id),
                (0.0, ih, id),
            ] {
                pts.push([x + dx, y + dy, z + dz]);
            }
        }
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for p in &pts {
            let (sx, sy) = project(*p);
            min_x = min_x.min(sx);
            max_x = max_x.max(sx);
            min_y = min_y.min(sy);
            max_y = max_y.max(sy);
        }
        (min_x, max_x, min_y, max_y)
    }

    /// 绘制箱体线框。
    fn draw_bin<DB: DrawingBackend>(&self, chart: &mut Chart<'_, DB>) -> io::Result<()> {
        let (w, h, d) = (self.bin.width, self.bin.height, self.bin.depth);
        let c: [[f64; 3]; 8] = [
            [0.0, 0.0, 0.0],
            [w, 0.0, 0.0],
            [w, h, 0.0],
            [0.0, h, 0.0],
            [0.0, 0.0, d],
            [w, 0.0, d],
            [w, h, d],
            [0.0, h, d],
        ];
        let edges: [(usize, usize); 12] = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        for (a, b) in edges {
            let line = vec![project(c[a]), project(c[b])];
            chart
                .draw_series(std::iter::once(PathElement::new(line, BLACK)))
                .map_err(io_err)?;
        }
        Ok(())
    }

    /// 绘制全部物品。
    fn draw_items<DB: DrawingBackend>(
        &self,
        chart: &mut Chart<'_, DB>,
        alpha: f64,
        write_num: bool,
        fontsize: u32,
    ) -> io::Result<()> {
        for item in &self.bin.items {
            let color = rgb_from_name(&item.color);
            let (x, y, z) = (item.position[0], item.position[1], item.position[2]);
            let dim = item.dimension();
            let text = if write_num { item.partno.as_str() } else { "" };
            match item.type_of {
                ItemType::Cube => {
                    self.draw_cube(chart, x, y, z, dim, color, alpha, text, fontsize)?
                }
                ItemType::Cylinder => {
                    self.draw_cylinder(chart, x, y, z, dim, color, alpha, text, fontsize)?
                }
            }
        }
        Ok(())
    }

    /// 绘制一个立方体（顶 + 两侧三个面 + 线框）。
    #[allow(clippy::too_many_arguments)]
    fn draw_cube<DB: DrawingBackend>(
        &self,
        chart: &mut Chart<'_, DB>,
        x: f64,
        y: f64,
        z: f64,
        dim: [f64; 3],
        color: RGBColor,
        alpha: f64,
        text: &str,
        fontsize: u32,
    ) -> io::Result<()> {
        let (w, h, d) = (dim[0], dim[1], dim[2]);
        let v: [[f64; 3]; 8] = [
            [x, y, z],
            [x + w, y, z],
            [x + w, y + h, z],
            [x, y + h, z],
            [x, y, z + d],
            [x + w, y, z + d],
            [x + w, y + h, z + d],
            [x, y + h, z + d],
        ];
        let fill = with_alpha(color, alpha);
        // 顶面 + 两个侧面
        for face in [[3usize, 2, 6, 7], [1, 2, 6, 5], [4, 5, 6, 7]] {
            let poly: Vec<(f64, f64)> = face.iter().map(|&i| project(v[i])).collect();
            chart
                .draw_series(std::iter::once(Polygon::new(poly, fill)))
                .map_err(io_err)?;
        }
        // 线框
        let edges: [(usize, usize); 12] = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        for (a, b) in edges {
            let line = vec![project(v[a]), project(v[b])];
            chart
                .draw_series(std::iter::once(PathElement::new(line, BLACK)))
                .map_err(io_err)?;
        }
        if !text.is_empty() {
            let cx = project([x + w / 2.0, y + h / 2.0, z + d / 2.0]);
            chart
                .draw_series(std::iter::once(Text::new(
                    text.to_owned(),
                    cx,
                    ("sans-serif", fontsize).into_font(),
                )))
                .map_err(io_err)?;
        }
        Ok(())
    }

    /// 绘制一个圆柱体示意（上下椭圆 + 侧面）。
    #[allow(clippy::too_many_arguments)]
    fn draw_cylinder<DB: DrawingBackend>(
        &self,
        chart: &mut Chart<'_, DB>,
        x: f64,
        y: f64,
        z: f64,
        dim: [f64; 3],
        color: RGBColor,
        alpha: f64,
        text: &str,
        fontsize: u32,
    ) -> io::Result<()> {
        let (w, h, d) = (dim[0], dim[1], dim[2]);
        let rx = w / 2.0;
        let rz = d / 2.0;
        let cx = x + rx;
        let cz = z + rz;
        let n = 24usize;
        let mut bottom: Vec<(f64, f64)> = Vec::with_capacity(n + 1);
        let mut top: Vec<(f64, f64)> = Vec::with_capacity(n + 1);
        for i in 0..n {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            let px = cx + rx * t.cos();
            let pz = cz + rz * t.sin();
            bottom.push(project([px, y, pz]));
            top.push(project([px, y + h, pz]));
        }
        bottom.push(bottom[0]);
        top.push(top[0]);

        let fill = with_alpha(color, alpha);
        // 侧面（下轮廓正向 + 上轮廓反向）
        let mut body: Vec<(f64, f64)> = bottom.clone();
        let mut top_rev: Vec<(f64, f64)> = top.iter().rev().cloned().collect();
        body.append(&mut top_rev);
        chart
            .draw_series(std::iter::once(Polygon::new(body, fill)))
            .map_err(io_err)?;
        // 顶面
        chart
            .draw_series(std::iter::once(Polygon::new(top.clone(), fill)))
            .map_err(io_err)?;
        // 轮廓
        chart
            .draw_series(std::iter::once(PathElement::new(bottom, BLACK)))
            .map_err(io_err)?;
        chart
            .draw_series(std::iter::once(PathElement::new(top, BLACK)))
            .map_err(io_err)?;

        if !text.is_empty() {
            let tc = project([cx, y + h / 2.0, cz]);
            chart
                .draw_series(std::iter::once(Text::new(
                    text.to_owned(),
                    tc,
                    ("sans-serif", fontsize).into_font(),
                )))
                .map_err(io_err)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::ItemType;
    use crate::item::Item;
    use crate::packer::{PackOptions, Packer};

    /// 打包 README 示例场景并渲染 PNG，断言成功且文件非空。
    #[test]
    fn plot_renders_nonempty_png() {
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

        let path = std::env::temp_dir().join("smartpacker_plot_smoke.png");
        let res = Painter::new(&p.bins[0]).plot_box_and_items("smoke", 0.8, true, 14, &path);
        assert!(res.is_ok(), "plot must render: {res:?}");
        let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        assert!(len > 0, "png must not be empty");
        let _ = std::fs::remove_file(&path);
    }
}
