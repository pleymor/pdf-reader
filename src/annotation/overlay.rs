use crate::pdf::models::*;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Render annotations for a single page into a transparent RGBA overlay.
pub fn render_overlay(
    annotations: &[Annotation],
    page_width_px: u32,
    page_height_px: u32,
    page_height_pt: f64,
    scale: f64,
) -> Option<Pixmap> {
    if annotations.is_empty() {
        return None;
    }

    let mut pixmap = Pixmap::new(page_width_px, page_height_px)?;

    for ann in annotations {
        match ann {
            Annotation::Rect(r) => draw_rect(&mut pixmap, r, page_height_pt, scale),
            Annotation::Circle(c) => draw_circle(&mut pixmap, c, page_height_pt, scale),
            Annotation::Text(t) => draw_text_annotation(&mut pixmap, t, page_height_pt, scale),
            Annotation::Signature(s) => draw_signature(&mut pixmap, s, page_height_pt, scale),
        }
    }

    Some(pixmap)
}

/// PDF coords (bottom-left, Y-up) to canvas coords (top-left, Y-down).
fn pdf_to_canvas(pdf_x: f64, pdf_y: f64, page_height_pt: f64, scale: f64) -> (f32, f32) {
    let cx = (pdf_x * scale) as f32;
    let cy = ((page_height_pt - pdf_y) * scale) as f32;
    (cx, cy)
}

fn rgb_paint(color: &RgbColor) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(color.r, color.g, color.b, 255));
    paint.anti_alias = true;
    paint
}

fn draw_rect(pixmap: &mut Pixmap, ann: &RectAnnotation, page_h: f64, scale: f64) {
    let (x1, y1) = pdf_to_canvas(ann.x, ann.y + ann.height, page_h, scale);
    let (x2, y2) = pdf_to_canvas(ann.x + ann.width, ann.y, page_h, scale);

    let left = x1.min(x2);
    let top = y1.min(y2);
    let w = (x2 - x1).abs();
    let h = (y2 - y1).abs();

    if let Some(rect) = tiny_skia::Rect::from_xywh(left, top, w, h) {
        let paint = rgb_paint(&ann.color);
        let stroke = Stroke {
            width: (ann.stroke_width * scale) as f32,
            ..Stroke::default()
        };
        let path = PathBuilder::from_rect(rect);
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn draw_circle(pixmap: &mut Pixmap, ann: &CircleAnnotation, page_h: f64, scale: f64) {
    let (x1, y1) = pdf_to_canvas(ann.x, ann.y + ann.height, page_h, scale);
    let (x2, y2) = pdf_to_canvas(ann.x + ann.width, ann.y, page_h, scale);

    let cx = (x1 + x2) / 2.0;
    let cy = (y1 + y2) / 2.0;
    let rx = (x2 - x1).abs() / 2.0;
    let ry = (y2 - y1).abs() / 2.0;

    if let Some(oval) = tiny_skia::Rect::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0) {
        let paint = rgb_paint(&ann.color);
        let stroke = Stroke {
            width: (ann.stroke_width * scale) as f32,
            ..Stroke::default()
        };
        if let Some(path) = PathBuilder::from_oval(oval) {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_text_annotation(pixmap: &mut Pixmap, ann: &TextAnnotation, page_h: f64, scale: f64) {
    let (x, y) = pdf_to_canvas(ann.x, ann.y, page_h, scale);
    let font_size_px = (ann.font_size * scale) as f32;
    let line_h = font_size_px * 1.2;
    let lines = ann.content.split('\n').count().max(1) as f32;
    let w = (ann.width * scale) as f32;
    let h = lines * line_h;

    // Draw semi-transparent background
    if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, w, h) {
        let mut bg_paint = Paint::default();
        bg_paint.set_color(Color::from_rgba8(255, 255, 255, 180));
        bg_paint.anti_alias = true;
        let path = PathBuilder::from_rect(rect);
        pixmap.fill_path(&path, &bg_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }

    // Draw border
    if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, w, h) {
        let mut border_paint = Paint::default();
        border_paint.set_color(Color::from_rgba8(ann.color.r, ann.color.g, ann.color.b, 150));
        border_paint.anti_alias = true;
        let stroke = Stroke { width: 1.0, ..Stroke::default() };
        let path = PathBuilder::from_rect(rect);
        pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);
    }

    // Draw colored line on the left edge as text color indicator
    if h > 2.0 {
        let mut pb = PathBuilder::new();
        pb.move_to(x + 2.0, y + 2.0);
        pb.line_to(x + 2.0, y + h - 2.0);
        if let Some(path) = pb.finish() {
            let paint = rgb_paint(&ann.color);
            let stroke = Stroke { width: 3.0, ..Stroke::default() };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_signature(pixmap: &mut Pixmap, ann: &SignatureAnnotation, page_h: f64, scale: f64) {
    let (x1, y1) = pdf_to_canvas(ann.x, ann.y + ann.height, page_h, scale);
    let (x2, y2) = pdf_to_canvas(ann.x + ann.width, ann.y, page_h, scale);

    let left = x1.min(x2);
    let top = y1.min(y2);
    let w = (x2 - x1).abs() as u32;
    let h = (y2 - y1).abs() as u32;

    if w == 0 || h == 0 { return; }

    let b64 = ann.image_data.splitn(2, ',').last().unwrap_or(&ann.image_data);
    let Ok(png_bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64) else { return };
    let Ok(img) = image::load_from_memory(&png_bytes) else { return };

    let resized = img.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();

    if let Some(src) = Pixmap::from_vec(rgba.into_raw(), tiny_skia::IntSize::from_wh(w, h).unwrap()) {
        pixmap.draw_pixmap(
            left as i32,
            top as i32,
            src.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}
