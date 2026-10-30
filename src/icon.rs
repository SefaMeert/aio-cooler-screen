//! The app icon, drawn in code: a dark rounded square with a gauge ring.

use std::path::Path;

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::{ExtendedColorType, RgbaImage};
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LinearGradient, Paint, PathBuilder, Pixmap, Point, SpreadMode, Stroke,
    Transform,
};

fn gradient(x0: f32, y0: f32, x1: f32, y1: f32, a: Color, b: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.anti_alias = true;
    if let Some(s) = LinearGradient::new(
        Point::from_xy(x0, y0),
        Point::from_xy(x1, y1),
        vec![GradientStop::new(0.0, a), GradientStop::new(1.0, b)],
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        p.shader = s;
    }
    p
}

pub fn render(size: u32) -> RgbaImage {
    let s = size as f32;
    let mut pm = Pixmap::new(size, size).expect("icon size");
    let (r, m) = (s * 0.22, s * 0.04);

    let mut pb = PathBuilder::new();
    pb.move_to(m + r, m);
    pb.line_to(s - m - r, m);
    pb.quad_to(s - m, m, s - m, m + r);
    pb.line_to(s - m, s - m - r);
    pb.quad_to(s - m, s - m, s - m - r, s - m);
    pb.line_to(m + r, s - m);
    pb.quad_to(m, s - m, m, s - m - r);
    pb.line_to(m, m + r);
    pb.quad_to(m, m, m + r, m);
    pb.close();
    if let Some(path) = pb.finish() {
        let bg = gradient(0.0, 0.0, s, s, Color::from_rgba8(30, 41, 59, 255), Color::from_rgba8(8, 12, 22, 255));
        pm.fill_path(&path, &bg, FillRule::Winding, Transform::identity(), None);
    }

    // 270° gauge ring, cyan to pink, gap at the bottom.
    let (c, rad) = (s / 2.0, s * 0.30);
    let mut pb = PathBuilder::new();
    let steps = 48;
    for i in 0..=steps {
        let a = (135.0 + 270.0 * i as f32 / steps as f32 * 0.82).to_radians();
        let (x, y) = (c + rad * a.cos(), c + rad * a.sin());
        if i == 0 {
            pb.move_to(x, y)
        } else {
            pb.line_to(x, y)
        }
    }
    if let Some(path) = pb.finish() {
        let ring = gradient(0.0, s, s, 0.0, Color::from_rgba8(56, 189, 248, 255), Color::from_rgba8(244, 63, 94, 255));
        let stroke = Stroke { width: s * 0.11, line_cap: LineCap::Round, ..Default::default() };
        pm.stroke_path(&path, &ring, &stroke, Transform::identity(), None);
    }
    if let Some(dot) = PathBuilder::from_circle(c, c, s * 0.09) {
        let mut p = Paint::default();
        p.set_color_rgba8(240, 244, 248, 255);
        p.anti_alias = true;
        pm.fill_path(&dot, &p, FillRule::Winding, Transform::identity(), None);
    }

    let mut img = RgbaImage::new(size, size);
    for (dst, src) in img.pixels_mut().zip(pm.pixels()) {
        let c = src.demultiply();
        *dst = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    img
}

/// Single-colour version of the icon (the gauge ring and dot) for the system tray.
pub fn render_mono(size: u32, rgb: [u8; 3]) -> RgbaImage {
    let s = size as f32;
    let mut pm = Pixmap::new(size, size).expect("icon size");
    let mut paint = Paint::default();
    paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], 255);
    paint.anti_alias = true;
    let (c, rad) = (s / 2.0, s * 0.36);
    let mut pb = PathBuilder::new();
    let steps = 48;
    for i in 0..=steps {
        let a = (135.0 + 270.0 * i as f32 / steps as f32 * 0.82).to_radians();
        let (x, y) = (c + rad * a.cos(), c + rad * a.sin());
        if i == 0 {
            pb.move_to(x, y)
        } else {
            pb.line_to(x, y)
        }
    }
    if let Some(path) = pb.finish() {
        let stroke = Stroke { width: s * 0.15, line_cap: LineCap::Round, ..Default::default() };
        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
    if let Some(dot) = PathBuilder::from_circle(c, c, s * 0.12) {
        pm.fill_path(&dot, &paint, FillRule::Winding, Transform::identity(), None);
    }
    let mut img = RgbaImage::new(size, size);
    for (dst, src) in img.pixels_mut().zip(pm.pixels()) {
        let c = src.demultiply();
        *dst = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_icon_is_single_colour() {
        let img = render_mono(32, [255, 255, 255]);
        assert!(img.pixels().filter(|p| p[3] > 0).all(|p| p[0] == 255 && p[1] == 255 && p[2] == 255));
        assert!(img.pixels().any(|p| p[3] == 255));
        if let Ok(dir) = std::env::var("CARGO_TARGET_TMPDIR") {
            let _ = img.save(std::path::Path::new(&dir).join("tray-dark-taskbar.png"));
            let _ = render_mono(32, [24, 24, 24]).save(std::path::Path::new(&dir).join("tray-light-taskbar.png"));
        }
    }
}

/// Write icon.png (256 px) and a multi-size icon.ico into `dir`.
pub fn write_assets(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    render(256).save(dir.join("icon.png")).map_err(|e| e.to_string())?;
    let images: Vec<RgbaImage> = [16, 24, 32, 48, 64, 128, 256].into_iter().map(render).collect();
    let frames = images
        .iter()
        .map(|i| IcoFrame::as_png(i.as_raw(), i.width(), i.height(), ExtendedColorType::Rgba8))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let file = std::fs::File::create(dir.join("icon.ico")).map_err(|e| e.to_string())?;
    IcoEncoder::new(file).encode_images(&frames).map_err(|e| e.to_string())
}
