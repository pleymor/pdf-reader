use pdfium_render::prelude::*;

/// A character with its bounding box in canvas pixel coordinates.
#[derive(Clone)]
pub struct CharBox {
    pub ch: char,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Extract character positions from a PDF page.
/// Returns chars with bounding boxes in canvas pixel coordinates.
pub fn extract_char_boxes(
    pdfium: &Pdfium,
    path: &std::path::Path,
    page_index: u16,
    scale: f32,
    page_height_pt: f32,
) -> Vec<CharBox> {
    let Ok(doc) = pdfium.load_pdf_from_file(path, None) else { return Vec::new() };
    let Ok(page) = doc.pages().get(page_index) else { return Vec::new() };
    let Ok(text) = page.text() else { return Vec::new() };

    let mut boxes = Vec::new();
    for ch in text.chars().iter() {
        let Some(unicode) = ch.unicode_char() else { continue };
        if unicode == '\0' || unicode.is_control() { continue; }

        if let Ok(rect) = ch.loose_bounds() {
            let left = rect.left.value * scale;
            let top = (page_height_pt - rect.top.value) * scale;
            let right = rect.right.value * scale;
            let bottom = (page_height_pt - rect.bottom.value) * scale;
            boxes.push(CharBox {
                ch: unicode,
                left,
                top: top.min(bottom),
                right,
                bottom: top.max(bottom),
            });
        }
    }
    boxes
}

/// Find the char index nearest to a canvas pixel position.
fn nearest_char_index(chars: &[CharBox], x: f32, y: f32) -> Option<usize> {
    if chars.is_empty() { return None; }
    let mut best_idx = 0;
    let mut best_dist = f32::MAX;
    for (i, ch) in chars.iter().enumerate() {
        let cx = (ch.left + ch.right) / 2.0;
        let cy = (ch.top + ch.bottom) / 2.0;
        let dist = (cx - x).powi(2) + (cy - y).powi(2);
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
        }
    }
    Some(best_idx)
}

/// Select text between two canvas pixel positions (start drag, end drag).
/// Returns (selected_text, Vec of highlight rects in canvas px).
pub fn select_text(
    chars: &[CharBox],
    x1: f32, y1: f32,
    x2: f32, y2: f32,
) -> (String, Vec<(f32, f32, f32, f32)>) {
    if chars.is_empty() { return (String::new(), Vec::new()); }

    let Some(start_idx) = nearest_char_index(chars, x1, y1) else {
        return (String::new(), Vec::new());
    };
    let Some(end_idx) = nearest_char_index(chars, x2, y2) else {
        return (String::new(), Vec::new());
    };

    let from = start_idx.min(end_idx);
    let to = start_idx.max(end_idx);

    let mut text = String::new();
    let mut rects: Vec<(f32, f32, f32, f32)> = Vec::new();
    let mut line_left = f32::MAX;
    let mut line_top = f32::MAX;
    let mut line_right = f32::MIN;
    let mut line_bottom = f32::MIN;
    let mut last_cy = -1.0_f32;

    for i in from..=to {
        let ch = &chars[i];
        let cy = (ch.top + ch.bottom) / 2.0;

        // Detect new line
        if last_cy > 0.0 && (cy - last_cy).abs() > (ch.bottom - ch.top) * 0.4 {
            // Flush current line rect
            if line_right > line_left {
                rects.push((line_left, line_top, line_right - line_left, line_bottom - line_top));
            }
            text.push('\n');
            line_left = f32::MAX;
            line_top = f32::MAX;
            line_right = f32::MIN;
            line_bottom = f32::MIN;
        }

        text.push(ch.ch);
        line_left = line_left.min(ch.left);
        line_top = line_top.min(ch.top);
        line_right = line_right.max(ch.right);
        line_bottom = line_bottom.max(ch.bottom);
        last_cy = cy;
    }

    // Flush last line
    if line_right > line_left {
        rects.push((line_left, line_top, line_right - line_left, line_bottom - line_top));
    }

    (text, rects)
}
