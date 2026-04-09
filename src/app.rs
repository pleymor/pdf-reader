use crate::annotation::{interaction::InteractionState, overlay, store::AnnotationStore};
use crate::viewer::{forms, links, text_selection};
use crate::pdf::writer;
use crate::{App, FormFieldData, PageData, TextSelRect};
use pdfium_render::prelude::*;
use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, Timer, TimerMode, VecModel};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// ── Zoom levels (same as TypeScript version) ─────────────────────────────────

const ZOOM_LEVELS: &[f32] = &[
    0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0,
];

fn snap_zoom(current: f32, dir: i32) -> f32 {
    if dir > 0 {
        ZOOM_LEVELS
            .iter()
            .find(|&&z| z > current + 0.005)
            .copied()
            .unwrap_or(*ZOOM_LEVELS.last().unwrap())
    } else {
        ZOOM_LEVELS
            .iter()
            .rev()
            .find(|&&z| z < current - 0.005)
            .copied()
            .unwrap_or(ZOOM_LEVELS[0])
    }
}

// ── Page dimensions ──────────────────────────────────────────────────────────

#[derive(Clone)]
struct PageDim {
    width_pt: f32,
    height_pt: f32,
}

// ── App state ────────────────────────────────────────────────────────────────

struct ViewerState {
    file_path: Option<PathBuf>,
    page_dims: Vec<PageDim>,
    page_count: u16,
    scale: f32,
    rotation: i32, // 0, 90, 180, 270
    annotations: AnnotationStore,
    interaction: InteractionState,
    dirty: bool,
    form_values: std::collections::HashMap<String, String>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            file_path: None,
            page_dims: Vec::new(),
            page_count: 0,
            scale: 1.5,
            rotation: 0,
            annotations: AnnotationStore::default(),
            interaction: InteractionState::default(),
            dirty: false,
            form_values: std::collections::HashMap::new(),
        }
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

fn render_page_to_image(
    pdfium: &Pdfium,
    path: &std::path::Path,
    page_index: u16,
    scale: f32,
    rotation: i32,
    annotations: &[crate::pdf::models::Annotation],
) -> Option<Image> {
    let doc = pdfium.load_pdf_from_file(path, None).ok()?;
    let page = doc.pages().get(page_index).ok()?;

    let pdfium_rotation = match rotation {
        90 => Some(PdfPageRenderRotation::Degrees90),
        180 => Some(PdfPageRenderRotation::Degrees180),
        270 => Some(PdfPageRenderRotation::Degrees270),
        _ => None,
    };

    let page_height_pt = page.height().value as f64;

    let (page_w, page_h) = if rotation == 90 || rotation == 270 {
        (page.height().value, page.width().value)
    } else {
        (page.width().value, page.height().value)
    };

    let width = (page_w * scale) as i32;
    let height = (page_h * scale) as i32;

    let mut config = PdfRenderConfig::new()
        .set_target_width(width)
        .set_target_height(height);

    if let Some(rot) = pdfium_rotation {
        config = config.rotate(rot, true);
    }

    let bitmap = page.render_with_config(&config).ok()?;
    let img = bitmap.as_image();
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    // Composite annotation overlay if any
    if !annotations.is_empty() {
        if let Some(overlay_pixmap) = overlay::render_overlay(
            annotations,
            w,
            h,
            page_height_pt,
            scale as f64,
        ) {
            let overlay_data = overlay_pixmap.data();
            let base = rgba.as_mut();
            // Alpha-blend overlay onto base
            for i in (0..base.len()).step_by(4) {
                let sa = overlay_data[i + 3] as u32;
                if sa == 0 { continue; }
                let da = 255 - sa;
                base[i]     = ((overlay_data[i] as u32 * sa + base[i] as u32 * da) / 255) as u8;
                base[i + 1] = ((overlay_data[i + 1] as u32 * sa + base[i + 1] as u32 * da) / 255) as u8;
                base[i + 2] = ((overlay_data[i + 2] as u32 * sa + base[i + 2] as u32 * da) / 255) as u8;
                base[i + 3] = (sa + base[i + 3] as u32 * da / 255) as u8;
            }
        }
    }

    let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    pixel_buffer.make_mut_bytes().copy_from_slice(&rgba);

    Some(Image::from_rgba8(pixel_buffer))
}

/// Build placeholders for all pages (no rendering yet).
fn build_page_placeholders(state: &ViewerState) -> (ModelRc<PageData>, Rc<VecModel<PageData>>) {
    let items: Vec<PageData> = state
        .page_dims
        .iter()
        .enumerate()
        .map(|(i, dim)| {
            let (w, h) = if state.rotation == 90 || state.rotation == 270 {
                (dim.height_pt, dim.width_pt)
            } else {
                (dim.width_pt, dim.height_pt)
            };
            PageData {
                image: Image::default(),
                width: w * state.scale,
                height: h * state.scale,
                page_num: (i + 1) as i32,
                rendered: false,
            }
        })
        .collect();

    let vm = Rc::new(VecModel::from(items));
    (ModelRc::from(vm.clone()), vm)
}

/// Render a single page and update the model in-place.
fn render_single_page(
    pdfium: &Pdfium,
    state: &ViewerState,
    vm: &VecModel<PageData>,
    page_idx: usize,
) {
    if page_idx >= state.page_dims.len() { return; }
    let page_num = (page_idx + 1) as u32;
    let page_anns = state.annotations.get_for_page(page_num);
    if let Some(path) = &state.file_path {
        if let Some(img) = render_page_to_image(pdfium, path, page_idx as u16, state.scale, state.rotation, page_anns) {
            let mut data = vm.row_data(page_idx).unwrap();
            data.image = img;
            data.rendered = true;
            vm.set_row_data(page_idx, data);
        }
    }
}

/// Render visible pages (based on scroll position).
fn render_visible_pages(
    pdfium: &Pdfium,
    state: &ViewerState,
    vm: &VecModel<PageData>,
    scroll_y: f32,
    viewport_h: f32,
) {
    let gap = 8.0_f32;
    let mut y = 0.0_f32;
    for (i, dim) in state.page_dims.iter().enumerate() {
        let h = if state.rotation == 90 || state.rotation == 270 {
            dim.width_pt
        } else {
            dim.height_pt
        };
        let page_h = h * state.scale;
        let page_bottom = y + page_h;

        // Page is visible if it overlaps [scroll_y, scroll_y + viewport_h]
        if page_bottom >= scroll_y - 200.0 && y <= scroll_y + viewport_h + 200.0 {
            // Only render if not already rendered
            if let Some(data) = vm.row_data(i) {
                if !data.rendered {
                    render_single_page(pdfium, state, vm, i);
                }
            }
        }
        y += page_h + gap;
    }
}

/// Rebuild all pages (for zoom/rotation changes).
fn rebuild_all_pages(
    pdfium: &Pdfium,
    state: &ViewerState,
    ui: &App,
) -> Rc<VecModel<PageData>> {
    let (model_rc, vm) = build_page_placeholders(state);
    ui.set_pages(model_rc);
    ui.set_zoom_text(format!("{}%", (state.scale * 100.0).round() as i32).into());

    // Render visible pages
    let scroll_y = ui.get_current_scroll_y();
    let viewport_h = ui.get_viewer_height();
    render_visible_pages(pdfium, state, &vm, scroll_y, viewport_h);
    vm
}

/// Re-render only one page (for annotation changes).
fn update_single_page(
    pdfium: &Pdfium,
    state: &ViewerState,
    vm: &VecModel<PageData>,
    page_num: u32,
) {
    let page_idx = (page_num - 1) as usize;
    render_single_page(pdfium, state, vm, page_idx);
}

// Legacy wrapper — rebuilds everything. Used for zoom/rotation.
fn update_ui(pdfium: &Pdfium, state: &ViewerState, ui: &App) {
    let (model_rc, vm) = build_page_placeholders(state);
    ui.set_pages(model_rc);
    ui.set_zoom_text(format!("{}%", (state.scale * 100.0).round() as i32).into());
    let scroll_y = ui.get_current_scroll_y();
    let viewport_h = ui.get_viewer_height();
    render_visible_pages(pdfium, state, &vm, scroll_y, viewport_h);
}

// ── Setup ────────────────────────────────────────────────────────────────────

pub fn setup(ui: &App) {
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library())
        .expect("Failed to load PDFium library. Place pdfium.dll next to the executable.");

    let pdfium = Arc::new(Pdfium::new(bindings));
    let state = Arc::new(Mutex::new(ViewerState::default()));
    // Shared page model (Rc because Slint is single-threaded)
    let pages_vm: std::rc::Rc<std::cell::RefCell<Option<Rc<VecModel<PageData>>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));

    // ── Open file ────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        let pages_vm = pages_vm.clone();
        ui.on_open_file(move || {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf", "PDF"])
                .add_filter("All", &["*"])
                .set_title("Open PDF")
                .pick_file()
            else {
                return;
            };

            let ui = ui_weak.unwrap();
            match load_document(&pdfium, &path) {
                Ok(loaded) => {
                    let page_count = loaded.dims.len() as u16;
                    let ann_count = loaded.annotations.len();
                    {
                        let mut s = state.lock().unwrap();
                        s.file_path = Some(path.clone());
                        s.page_dims = loaded.dims;
                        s.page_count = page_count;
                        s.rotation = 0;
                        s.annotations.load(loaded.annotations);
                    }

                    let s = state.lock().unwrap();
                    let vm = rebuild_all_pages(&pdfium, &s, &ui);
                    *pages_vm.borrow_mut() = Some(vm);
                    ui.set_page_count(page_count as i32);
                    ui.set_current_page(1);
                    ui.set_page_text("1".into());
                    ui.set_has_document(true);

                    // Extract form fields for all pages
                    // Also read values from lopdf (pdfium may not see values written by lopdf)
                    let lopdf_values = read_lopdf_form_values(&path);

                    let mut all_form_fields: Vec<FormFieldData> = Vec::new();
                    for i in 0..page_count {
                        let page_height_pt = s.page_dims.get(i as usize)
                            .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                            .unwrap_or(841.0);
                        let ff = forms::extract_form_fields(&pdfium, &path, i, s.scale, page_height_pt);
                        for f in ff {
                            // Prefer lopdf value over pdfium value (handles post-save reload)
                            let value = lopdf_values.get(&f.name)
                                .cloned()
                                .unwrap_or(f.value.clone());
                            let checked = if f.field_type == forms::FormFieldType::CheckBox {
                                value == "true" || value == "Yes" || value == "On"
                            } else {
                                f.checked
                            };
                            all_form_fields.push(FormFieldData {
                                name: f.name.clone().into(),
                                field_type: match f.field_type {
                                    forms::FormFieldType::Text => "text".into(),
                                    forms::FormFieldType::CheckBox => "checkbox".into(),
                                    forms::FormFieldType::Radio => "radio".into(),
                                    forms::FormFieldType::Dropdown => "dropdown".into(),
                                },
                                page_num: (f.page + 1) as i32,
                                x: f.left,
                                y: f.top,
                                w: f.width,
                                h: f.height,
                                value: value.into(),
                                checked,
                            });
                        }
                    }
                    if !all_form_fields.is_empty() {
                        ui.set_form_fields(ModelRc::new(VecModel::from(all_form_fields)));
                    }

                    let status = if ann_count > 0 {
                        format!(
                            "{} — {} pages, {} annotations",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            page_count,
                            ann_count
                        )
                    } else {
                        format!(
                            "{} — {} pages",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            page_count
                        )
                    };
                    ui.set_status_text(SharedString::from(status));
                }
                Err(e) => {
                    ui.set_status_text(format!("Error: {}", e).into());
                }
            }
        });
    }

    // ── Zoom in ──────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_zoom_in(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if s.page_count == 0 { return; }
            s.scale = snap_zoom(s.scale, 1);
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Zoom out ─────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_zoom_out(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if s.page_count == 0 { return; }
            s.scale = snap_zoom(s.scale, -1);
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Fit width ────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_fit_width(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if s.page_count == 0 { return; }
            let container_width = ui.get_viewer_width() - 60.0;
            let ref_width = if s.rotation == 90 || s.rotation == 270 {
                s.page_dims[0].height_pt
            } else {
                s.page_dims[0].width_pt
            };
            s.scale = (container_width / ref_width).clamp(0.25, 5.0);
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Fit height ───────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_fit_height(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if s.page_count == 0 { return; }
            let container_height = ui.get_viewer_height() - 80.0;
            let ref_height = if s.rotation == 90 || s.rotation == 270 {
                s.page_dims[0].width_pt
            } else {
                s.page_dims[0].height_pt
            };
            s.scale = (container_height / ref_height).clamp(0.25, 5.0);
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Rotate ───────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_rotate(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if s.page_count == 0 { return; }
            s.rotation = (s.rotation + 90) % 360;
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Page navigation ──────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_page_prev(move || {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            let cur = ui.get_current_page();
            if cur > 1 {
                let new_page = cur - 1;
                ui.set_current_page(new_page);
                ui.set_page_text(new_page.to_string().into());
                scroll_to_page(&ui, &s, new_page);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_page_next(move || {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            let cur = ui.get_current_page();
            if cur < s.page_count as i32 {
                let new_page = cur + 1;
                ui.set_current_page(new_page);
                ui.set_page_text(new_page.to_string().into());
                scroll_to_page(&ui, &s, new_page);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_page_goto(move |page| {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            let clamped = page.max(1).min(s.page_count as i32);
            ui.set_current_page(clamped);
            ui.set_page_text(clamped.to_string().into());
            scroll_to_page(&ui, &s, clamped);
        });
    }

    // ── Tool switching ─────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_set_tool(move |tool| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            s.interaction.tool = crate::annotation::interaction::Tool::from_str(&tool);
            ui.set_active_tool(tool);
        });
    }

    // ── Pointer events on pages ─────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_page_pointer_down(move |page, x, y| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            s.interaction.pointer_down(page as u32, x, y);
            ui.set_drawing(true);
            ui.set_drawing_page(page);
            ui.set_draw_x(x.min(x));
            ui.set_draw_y(y.min(y));
            ui.set_draw_w(0.0);
            ui.set_draw_h(0.0);
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_page_pointer_move(move |_page, x, y| {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            if s.interaction.drawing {
                let sx = s.interaction.start_x;
                let sy = s.interaction.start_y;
                ui.set_draw_x(sx.min(x));
                ui.set_draw_y(sy.min(y));
                ui.set_draw_w((x - sx).abs());
                ui.set_draw_h((y - sy).abs());
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_page_pointer_up(move |page, x, y| {
            let ui = ui_weak.unwrap();
            ui.set_drawing(false);
            let mut s = state.lock().unwrap();
            let page_idx = (page - 1) as usize;
            let page_height_pt = s.page_dims.get(page_idx)
                .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                .unwrap_or(841.0);
            let scale = s.scale;
            let is_text_tool = s.interaction.tool == crate::annotation::interaction::Tool::Text;

            if let Some(ann) = s.interaction.pointer_up(
                page as u32, x, y, scale, page_height_pt,
            ) {
                if is_text_tool {
                    // Store pending text annotation, show dialog
                    s.interaction.pending_text_ann = Some(ann);
                    ui.set_text_input_value("".into());
                    ui.set_show_text_input(true);
                } else {
                    s.annotations.add(ann);
                    s.dirty = true;
                    update_ui(&pdfium, &s, &ui);
                }
            }
        });
    }

    // ── Click in select mode (hit test) ─────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_page_click(move |page, x, y| {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            let page_idx = (page - 1) as usize;
            let page_height_pt = s.page_dims.get(page_idx)
                .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                .unwrap_or(841.0);
            let scale = s.scale;

            // Check if clicking a resize handle first
            if s.interaction.selected_idx.is_some()
                && s.interaction.selected_page == page as u32
                && ui.get_has_selection()
            {
                let sx = ui.get_sel_x();
                let sy = ui.get_sel_y();
                let sw = ui.get_sel_w();
                let sh = ui.get_sel_h();
                let handle_r = 10.0_f32;

                // NW
                if (x - sx).abs() < handle_r && (y - sy).abs() < handle_r {
                    drop(s);
                    ui.invoke_resize_start(0, page, sx + sw, sy + sh);
                    return;
                }
                // NE
                if (x - (sx + sw)).abs() < handle_r && (y - sy).abs() < handle_r {
                    drop(s);
                    ui.invoke_resize_start(1, page, sx, sy + sh);
                    return;
                }
                // SW
                if (x - sx).abs() < handle_r && (y - (sy + sh)).abs() < handle_r {
                    drop(s);
                    ui.invoke_resize_start(2, page, sx + sw, sy);
                    return;
                }
                // SE
                if (x - (sx + sw)).abs() < handle_r && (y - (sy + sh)).abs() < handle_r {
                    drop(s);
                    ui.invoke_resize_start(3, page, sx, sy);
                    return;
                }
            }

            // Hit test: find annotation under click point
            let anns = s.annotations.get_for_page(page as u32);
            if let Some((idx, bounds)) = hit_test(anns, x, y, scale as f64, page_height_pt as f64) {
                let ann = &anns[idx];
                let type_str = ann_type_str(ann);
                // Sync UI style from selected annotation
                let (cr, cg, cb) = match ann {
                    crate::pdf::models::Annotation::Rect(r) => (r.color.r, r.color.g, r.color.b),
                    crate::pdf::models::Annotation::Circle(c) => (c.color.r, c.color.g, c.color.b),
                    crate::pdf::models::Annotation::Text(t) => (t.color.r, t.color.g, t.color.b),
                    crate::pdf::models::Annotation::Signature(_) => (0, 0, 0),
                };
                let sw = match ann {
                    crate::pdf::models::Annotation::Rect(r) => r.stroke_width,
                    crate::pdf::models::Annotation::Circle(c) => c.stroke_width,
                    _ => 2.0,
                };
                let fs = match ann {
                    crate::pdf::models::Annotation::Text(t) => t.font_size,
                    _ => 14.0,
                };

                drop(s);
                let mut s = state.lock().unwrap();
                s.interaction.selected_idx = Some(idx);
                s.interaction.selected_page = page as u32;
                s.interaction.dragging = false;
                ui.set_has_selection(true);
                ui.set_selection_page(page);
                ui.set_sel_x(bounds.0);
                ui.set_sel_y(bounds.1);
                ui.set_sel_w(bounds.2);
                ui.set_sel_h(bounds.3);
                ui.set_selection_type(type_str.into());
                ui.set_cur_r(cr as i32);
                ui.set_cur_g(cg as i32);
                ui.set_cur_b(cb as i32);
                ui.set_cur_stroke_width(sw as i32);
                ui.set_cur_font_size(fs as i32);
            } else {
                drop(s);
                let mut s = state.lock().unwrap();
                s.interaction.clear_selection();
                ui.set_has_selection(false);
                ui.set_selection_type("".into());
            }
        });
    }

    // ── Drag move (select mode) ─────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_page_drag_move(move |page, x, y| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            let Some(idx) = s.interaction.selected_idx else { return };
            let sel_page = s.interaction.selected_page;
            if sel_page != page as u32 { return; }
            let scale = s.scale;

            if !s.interaction.dragging {
                let ann_clone = s.annotations.get_for_page(sel_page).get(idx).cloned();
                if let Some(ann) = ann_clone {
                    s.interaction.start_drag(x, y, &ann);
                }
                return;
            }

            // Compute new selection box position from drag delta (visual only, no re-render)
            let dx_px = x - s.interaction.drag_start_x;
            let dy_px = y - s.interaction.drag_start_y;
            let page_height_pt = s.page_dims.get((page - 1) as usize)
                .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                .unwrap_or(841.0);

            // Get original bounds to compute visual offset
            let orig_x = s.interaction.drag_orig_pdf_x;
            let orig_y = s.interaction.drag_orig_pdf_y;
            let dx_pdf = dx_px as f64 / scale as f64;
            let dy_pdf = -(dy_px as f64) / scale as f64;

            // Temporarily compute where the annotation would be
            let ann_clone = s.annotations.get_for_page(sel_page).get(idx).cloned();
            if let Some(mut tmp_ann) = ann_clone {
                crate::annotation::interaction::set_ann_origin(&mut tmp_ann, orig_x + dx_pdf, orig_y + dy_pdf);
                let bounds = ann_canvas_bounds(&tmp_ann, scale as f64, page_height_pt as f64);
                ui.set_sel_x(bounds.0);
                ui.set_sel_y(bounds.1);
                ui.set_sel_w(bounds.2);
                ui.set_sel_h(bounds.3);
            }
        });
    }

    // ── Drag end (select mode) ──────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_page_drag_end(move |page, x, y| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if !s.interaction.dragging {
                s.interaction.end_drag();
                return;
            }
            let Some(idx) = s.interaction.selected_idx else {
                s.interaction.end_drag();
                return;
            };
            let sel_page = s.interaction.selected_page;
            let scale = s.scale;

            // Apply final position
            let dx_pdf = (x - s.interaction.drag_start_x) as f64 / scale as f64;
            let dy_pdf = -(y - s.interaction.drag_start_y) as f64 / scale as f64;
            let orig_x = s.interaction.drag_orig_pdf_x;
            let orig_y = s.interaction.drag_orig_pdf_y;

            if let Some(anns) = s.annotations.get_mut_for_page(sel_page) {
                if let Some(ann) = anns.get_mut(idx) {
                    crate::annotation::interaction::set_ann_origin(ann, orig_x + dx_pdf, orig_y + dy_pdf);
                }
            }
            s.interaction.end_drag();
            s.dirty = true;
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Delete selected ─────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_delete_selected(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            let Some(idx) = s.interaction.selected_idx else { return };
            let page = s.interaction.selected_page;
            s.annotations.remove(page, idx);
            s.interaction.clear_selection();
            s.dirty = true;
            ui.set_has_selection(false);
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Set color ────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_set_color(move |r, g, b| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            let color = crate::pdf::models::RgbColor { r: r as u8, g: g as u8, b: b as u8 };
            s.interaction.color = color.clone();
            ui.set_cur_r(r);
            ui.set_cur_g(g);
            ui.set_cur_b(b);

            // Apply to selected annotation if any
            if let Some(idx) = s.interaction.selected_idx {
                let page = s.interaction.selected_page;
                if let Some(anns) = s.annotations.get_mut_for_page(page) {
                    if let Some(ann) = anns.get_mut(idx) {
                        set_ann_color(ann, &color);
                        s.dirty = true;
                    }
                }
                update_ui(&pdfium, &s, &ui);
            }
        });
    }

    // ── Set stroke width ─────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_set_stroke_width(move |w| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            s.interaction.stroke_width = w as f64;
            ui.set_cur_stroke_width(w as i32);

            if let Some(idx) = s.interaction.selected_idx {
                let page = s.interaction.selected_page;
                if let Some(anns) = s.annotations.get_mut_for_page(page) {
                    if let Some(ann) = anns.get_mut(idx) {
                        set_ann_stroke_width(ann, w as f64);
                        s.dirty = true;
                    }
                }
                update_ui(&pdfium, &s, &ui);
            }
        });
    }

    // ── Set font size ────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_set_font_size(move |fs| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            s.interaction.font_size = fs as f64;
            ui.set_cur_font_size(fs as i32);

            if let Some(idx) = s.interaction.selected_idx {
                let page = s.interaction.selected_page;
                if let Some(anns) = s.annotations.get_mut_for_page(page) {
                    if let Some(ann) = anns.get_mut(idx) {
                        if let crate::pdf::models::Annotation::Text(t) = ann {
                            t.font_size = fs as f64;
                            s.dirty = true;
                        }
                    }
                }
                update_ui(&pdfium, &s, &ui);
            }
        });
    }

    // ── Resize ───────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_resize_start(move |_handle, page, anchor_x, anchor_y| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            if s.interaction.selected_idx.is_none() { return; }
            s.interaction.dragging = false;
            // Store anchor (the fixed corner) in canvas px
            s.interaction.drag_start_x = anchor_x;
            s.interaction.drag_start_y = anchor_y;
            s.interaction.start_page = page as u32;
            ui.set_resizing(true);
        });
    }
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_resize_move(move |mouse_x, mouse_y| {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            if s.interaction.selected_idx.is_none() { return; }

            // Anchor is the fixed corner, mouse is the moving corner
            let ax = s.interaction.drag_start_x;
            let ay = s.interaction.drag_start_y;

            // Update selection box visually (no re-render)
            let left_px = ax.min(mouse_x);
            let top_px = ay.min(mouse_y);
            let w_px = (mouse_x - ax).abs().max(10.0);
            let h_px = (mouse_y - ay).abs().max(10.0);

            ui.set_sel_x(left_px);
            ui.set_sel_y(top_px);
            ui.set_sel_w(w_px);
            ui.set_sel_h(h_px);
        });
    }
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_resize_end(move || {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            let Some(idx) = s.interaction.selected_idx else {
                ui.set_resizing(false);
                return;
            };
            let page = s.interaction.selected_page;
            let scale = s.scale;
            let page_idx = (page - 1) as usize;
            let page_height_pt = s.page_dims.get(page_idx)
                .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                .unwrap_or(841.0) as f64;

            // Read final selection box from UI
            let left_px = ui.get_sel_x();
            let top_px = ui.get_sel_y();
            let w_px = ui.get_sel_w();
            let h_px = ui.get_sel_h();

            let pdf_left = left_px as f64 / scale as f64;
            let pdf_bottom = page_height_pt - (top_px as f64 + h_px as f64) / scale as f64;
            let pdf_w = w_px as f64 / scale as f64;
            let pdf_h = h_px as f64 / scale as f64;

            if pdf_w > 5.0 && pdf_h > 5.0 {
                if let Some(anns) = s.annotations.get_mut_for_page(page) {
                    if let Some(ann) = anns.get_mut(idx) {
                        match ann {
                            crate::pdf::models::Annotation::Rect(r) => {
                                r.x = pdf_left; r.y = pdf_bottom; r.width = pdf_w; r.height = pdf_h;
                            }
                            crate::pdf::models::Annotation::Circle(c) => {
                                c.x = pdf_left; c.y = pdf_bottom; c.width = pdf_w; c.height = pdf_h;
                            }
                            crate::pdf::models::Annotation::Text(t) => {
                                t.x = pdf_left; t.y = pdf_bottom + pdf_h; t.width = pdf_w;
                            }
                            crate::pdf::models::Annotation::Signature(sig) => {
                                sig.x = pdf_left; sig.y = pdf_bottom; sig.width = pdf_w; sig.height = pdf_h;
                            }
                        }
                    }
                }
            }

            s.dirty = true;
            ui.set_resizing(false);
            update_ui(&pdfium, &s, &ui);
        });
    }

    // ── Save ─────────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_file(move || {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            let Some(path) = &s.file_path else { return };
            let all_annotations = s.annotations.all();
            let form_values = s.form_values.clone();

            match save_annotated_pdf(path, &all_annotations, &form_values) {
                Ok(()) => {
                    ui.set_status_text(SharedString::from("Saved"));
                }
                Err(e) => {
                    ui.set_status_text(SharedString::from(format!("Save error: {}", e)));
                }
            }
        });
    }

    // ── Text input dialog ──────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        ui.on_text_input_submit(move |text| {
            let ui = ui_weak.unwrap();
            ui.set_show_text_input(false);
            let mut s = state.lock().unwrap();
            if let Some(mut ann) = s.interaction.pending_text_ann.take() {
                if !text.is_empty() {
                    if let crate::pdf::models::Annotation::Text(ref mut t) = ann {
                        t.content = text.to_string();
                    }
                    s.annotations.add(ann);
                    s.dirty = true;
                    update_ui(&pdfium, &s, &ui);
                }
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_text_input_cancel(move || {
            let ui = ui_weak.unwrap();
            ui.set_show_text_input(false);
            let mut s = state.lock().unwrap();
            s.interaction.pending_text_ann = None;
        });
    }

    // ── Signature ─────────────────────────────────────────────────────────────
    {
        // High-res canvas (2x the display size for crisp rendering)
        const SIG_W: u32 = 960;
        const SIG_H: u32 = 360;
        // Scale factor: Slint modal draws at ~480x180, canvas is 2x
        const SIG_SCALE: f32 = 2.0;

        let sig_pixmap: std::rc::Rc<std::cell::RefCell<tiny_skia::Pixmap>> =
            std::rc::Rc::new(std::cell::RefCell::new(tiny_skia::Pixmap::new(SIG_W, SIG_H).unwrap()));
        let sig_b64: std::rc::Rc<std::cell::RefCell<Option<String>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));
        let sig_draw_count: std::rc::Rc<std::cell::Cell<u32>> =
            std::rc::Rc::new(std::cell::Cell::new(0));
        // Store all stroke points for smooth re-rendering
        // Each stroke is a Vec of (x, y) in high-res canvas coords
        let sig_strokes: std::rc::Rc<std::cell::RefCell<Vec<Vec<(f32, f32)>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

        {
            let ui_weak = ui.as_weak();
            let sig_pixmap = sig_pixmap.clone();
            let sig_strokes = sig_strokes.clone();
            ui.on_open_signature_modal(move || {
                let ui = ui_weak.unwrap();
                let mut pm = sig_pixmap.borrow_mut();
                *pm = tiny_skia::Pixmap::new(SIG_W, SIG_H).unwrap();
                sig_strokes.borrow_mut().clear();
                update_sig_image(&ui, &pm);
                ui.set_show_signature_modal(true);
            });
        }
        {
            let ui_weak = ui.as_weak();
            let sig_pixmap = sig_pixmap.clone();
            let sig_draw_count = sig_draw_count.clone();
            let sig_strokes = sig_strokes.clone();
            ui.on_sig_draw(move |x1, y1, x2, y2| {
                let mut pm = sig_pixmap.borrow_mut();

                let sx2 = x2 * SIG_SCALE;
                let sy2 = y2 * SIG_SCALE;
                let sx1 = x1 * SIG_SCALE;
                let sy1 = y1 * SIG_SCALE;

                // Collect point for smooth re-rendering later
                {
                    let mut strokes = sig_strokes.borrow_mut();
                    if strokes.is_empty() || strokes.last().map_or(true, |s| s.is_empty()) {
                        strokes.push(vec![(sx1, sy1)]);
                    }
                    if let Some(current) = strokes.last_mut() {
                        current.push((sx2, sy2));
                    }
                }

                // Draw rough preview line
                let dx = sx2 - sx1;
                let dy = sy2 - sy1;
                let angle = dy.atan2(dx);
                let nib_angle = std::f32::consts::FRAC_PI_4;
                let cross = (angle - nib_angle).sin().abs();
                let width = 1.5 + cross * 5.0;

                let mut paint = tiny_skia::Paint::default();
                paint.set_color(tiny_skia::Color::from_rgba8(15, 15, 35, 255));
                paint.anti_alias = true;
                let stroke = tiny_skia::Stroke {
                    width,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..tiny_skia::Stroke::default()
                };
                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(sx1, sy1);
                pb.line_to(sx2, sy2);
                if let Some(path) = pb.finish() {
                    pm.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
                }

                let count = sig_draw_count.get() + 1;
                sig_draw_count.set(count);
                if count % 3 == 0 {
                    update_sig_image(&ui_weak.unwrap(), &pm);
                }
            });
        }
        {
            let ui_weak = ui.as_weak();
            let sig_pixmap = sig_pixmap.clone();
            let sig_strokes = sig_strokes.clone();
            ui.on_sig_draw_end(move || {
                let ui = ui_weak.unwrap();
                // Mark end of current stroke
                sig_strokes.borrow_mut().push(Vec::new());
                // Re-render all strokes with smooth Bezier curves
                let mut pm = sig_pixmap.borrow_mut();
                *pm = tiny_skia::Pixmap::new(SIG_W, SIG_H).unwrap();
                render_smooth_strokes(&mut pm, &sig_strokes.borrow());
                update_sig_image(&ui, &pm);
            });
        }
        {
            let ui_weak = ui.as_weak();
            let sig_pixmap = sig_pixmap.clone();
            let sig_strokes = sig_strokes.clone();
            ui.on_sig_clear(move || {
                let ui = ui_weak.unwrap();
                let mut pm = sig_pixmap.borrow_mut();
                *pm = tiny_skia::Pixmap::new(SIG_W, SIG_H).unwrap();
                sig_strokes.borrow_mut().clear();
                update_sig_image(&ui, &pm);
            });
        }
        {
            let ui_weak = ui.as_weak();
            let sig_pixmap = sig_pixmap.clone();
            ui.on_sig_upload(move || {
                let ui = ui_weak.unwrap();
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg"])
                    .pick_file()
                else { return };
                if let Ok(img) = image::open(&path) {
                    let resized = img.resize(SIG_W, SIG_H, image::imageops::FilterType::Lanczos3);
                    let rgba = resized.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let mut pm = sig_pixmap.borrow_mut();
                    *pm = tiny_skia::Pixmap::new(SIG_W, SIG_H).unwrap();
                    if let Some(src) = tiny_skia::Pixmap::from_vec(
                        rgba.into_raw(), tiny_skia::IntSize::from_wh(w, h).unwrap(),
                    ) {
                        let dx = ((SIG_W - w) / 2) as i32;
                        let dy = ((SIG_H - h) / 2) as i32;
                        pm.draw_pixmap(dx, dy, src.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
                    }
                    update_sig_image(&ui, &pm);
                }
            });
        }
        {
            let ui_weak = ui.as_weak();
            let sig_pixmap = sig_pixmap.clone();
            let sig_b64 = sig_b64.clone();
            ui.on_sig_place(move || {
                let ui = ui_weak.unwrap();
                let pm = sig_pixmap.borrow();
                // Flush final image
                update_sig_image(&ui, &pm);
                let has_content = pm.data().chunks(4).any(|px| px[3] > 0);
                if !has_content { return; }
                let img = image::RgbaImage::from_raw(SIG_W, SIG_H, pm.data().to_vec()).unwrap();
                let mut png_buf: Vec<u8> = Vec::new();
                image::DynamicImage::ImageRgba8(img)
                    .write_to(&mut std::io::Cursor::new(&mut png_buf), image::ImageFormat::Png)
                    .unwrap();
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_buf);
                *sig_b64.borrow_mut() = Some(b64);
                ui.set_show_signature_modal(false);
                ui.set_sig_placing(true);
            });
        }
        {
            let ui_weak = ui.as_weak();
            ui.on_sig_cancel(move || {
                ui_weak.unwrap().set_show_signature_modal(false);
            });
        }
        {
            let ui_weak = ui.as_weak();
            let pdfium = pdfium.clone();
            let state = state.clone();
            let sig_b64 = sig_b64.clone();
            ui.on_sig_click_on_page(move |page, x, y| {
                let ui = ui_weak.unwrap();
                ui.set_sig_placing(false);
                let Some(b64) = sig_b64.borrow_mut().take() else { return };
                let mut s = state.lock().unwrap();
                let page_idx = (page - 1) as usize;
                let page_height_pt = s.page_dims.get(page_idx)
                    .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                    .unwrap_or(841.0);
                let scale = s.scale;
                let pdf_x = x as f64 / scale as f64;
                let pdf_y = page_height_pt as f64 - y as f64 / scale as f64;
                let ann = crate::pdf::models::Annotation::Signature(
                    crate::pdf::models::SignatureAnnotation {
                        page: page as u32,
                        x: pdf_x - 75.0,
                        y: pdf_y - 30.0,
                        width: 150.0,
                        height: 60.0,
                        image_data: b64,
                    },
                );
                s.annotations.add(ann);
                s.dirty = true;
                update_ui(&pdfium, &s, &ui);
            });
        }
    }

    // ── Save As ──────────────────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_file_as(move || {
            let ui = ui_weak.unwrap();
            let s = state.lock().unwrap();
            let Some(src_path) = s.file_path.clone() else { return };
            let all_annotations = s.annotations.all();
            let form_values = s.form_values.clone();
            drop(s);

            let Some(dest) = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_title("Save As")
                .set_file_name(
                    src_path.file_name().unwrap_or_default().to_string_lossy().to_string()
                )
                .save_file()
            else { return };

            // Copy original file to destination first, then save annotations
            if dest != *src_path {
                if let Err(e) = std::fs::copy(src_path, &dest) {
                    ui.set_status_text(SharedString::from(format!("Copy error: {}", e)));
                    return;
                }
            }

            match save_annotated_pdf(&dest, &all_annotations, &form_values) {
                Ok(()) => {
                    ui.set_status_text(SharedString::from(format!(
                        "Saved as {}",
                        dest.file_name().unwrap_or_default().to_string_lossy()
                    )));
                }
                Err(e) => {
                    ui.set_status_text(SharedString::from(format!("Save error: {}", e)));
                }
            }
        });
    }

    // ── Form field changes ─────────────────────────────────────────────────
    {
        let state = state.clone();
        ui.on_form_field_changed(move |name, value| {
            let mut s = state.lock().unwrap();
            s.form_values.insert(name.to_string(), value.to_string());
            s.dirty = true;
            // Don't update the Slint model — it would recreate the LineEdit and lose cursor/focus
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_form_checkbox_changed(move |name, checked| {
            let ui = ui_weak.unwrap();
            let mut s = state.lock().unwrap();
            s.form_values.insert(name.to_string(), if checked { "true".into() } else { "false".into() });
            s.dirty = true;

            let model = ui.get_form_fields();
            for i in 0..model.row_count() {
                if let Some(mut ff) = model.row_data(i) {
                    if ff.name == name {
                        ff.checked = checked;
                        ff.value = if checked { "true".into() } else { "false".into() };
                        model.set_row_data(i, ff);
                        break;
                    }
                }
            }
        });
    }

    // ── Text selection (read-only mode) ────────────────────────────────────
    {
        // Cache char boxes per page
        let text_cache: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<u16, Vec<text_selection::CharBox>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new()));
        let link_cache: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<u16, Vec<links::LinkBox>>>> =
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::HashMap::new()));
        let text_sel_start: std::rc::Rc<std::cell::Cell<(f32, f32)>> =
            std::rc::Rc::new(std::cell::Cell::new((0.0, 0.0)));
        let selected_text: std::rc::Rc<std::cell::RefCell<String>> =
            std::rc::Rc::new(std::cell::RefCell::new(String::new()));

        {
            let ui_weak = ui.as_weak();
            let text_sel_start = text_sel_start.clone();
            ui.on_text_select_start(move |_page, x, y| {
                let ui = ui_weak.unwrap();
                text_sel_start.set((x, y));
                ui.set_has_text_selection(false);
            });
        }
        // Helper to ensure char cache is populated for a page
        fn ensure_char_cache(
            cache: &mut std::collections::HashMap<u16, Vec<text_selection::CharBox>>,
            pdfium: &Pdfium,
            state: &ViewerState,
            page_idx: u16,
        ) {
            if cache.contains_key(&page_idx) { return; }
            if let Some(path) = &state.file_path {
                let page_height_pt = state.page_dims.get(page_idx as usize)
                    .map(|d| if state.rotation == 90 || state.rotation == 270 { d.width_pt } else { d.height_pt })
                    .unwrap_or(841.0);
                let chars = text_selection::extract_char_boxes(
                    pdfium, path, page_idx, state.scale, page_height_pt,
                );
                cache.insert(page_idx, chars);
            }
        }

        {
            let ui_weak = ui.as_weak();
            let pdfium = pdfium.clone();
            let state = state.clone();
            let text_cache = text_cache.clone();
            let text_sel_start = text_sel_start.clone();
            let selected_text = selected_text.clone();
            ui.on_text_select_move(move |page, x, y| {
                let ui = ui_weak.unwrap();
                let (sx, sy) = text_sel_start.get();
                if (x - sx).abs() < 5.0 && (y - sy).abs() < 5.0 { return; }

                let s = state.lock().unwrap();
                let page_idx = (page - 1) as u16;
                let mut cache = text_cache.borrow_mut();
                ensure_char_cache(&mut cache, &pdfium, &s, page_idx);

                if let Some(chars) = cache.get(&page_idx) {
                    let (text, rects) = text_selection::select_text(chars, sx, sy, x, y);
                    if !rects.is_empty() {
                        ui.set_has_text_selection(true);
                        ui.set_text_sel_page(page);
                        let rect_model: Vec<_> = rects.iter().map(|&(rx, ry, rw, rh)| {
                            TextSelRect { x: rx, y: ry, w: rw, h: rh }
                        }).collect();
                        ui.set_text_sel_rects(slint::ModelRc::new(slint::VecModel::from(rect_model)));
                        *selected_text.borrow_mut() = text;
                    }
                }
            });
        }
        {
            let ui_weak = ui.as_weak();
            let text_sel_start = text_sel_start.clone();
            let link_cache = link_cache.clone();
            ui.on_text_select_end(move |page, x, y| {
                let ui = ui_weak.unwrap();
                let (sx, sy) = text_sel_start.get();
                if (x - sx).abs() < 5.0 && (y - sy).abs() < 5.0 {
                    // It was a click — check for link
                    ui.set_has_text_selection(false);
                    let page_idx = (page - 1) as u16;
                    let lcache = link_cache.borrow();
                    if let Some(page_links) = lcache.get(&page_idx) {
                        if let Some(link) = links::link_at(page_links, x, y) {
                            let _ = open::that(&link.url);
                        }
                    }
                }
            });
        }
        // Hover: determine cursor type (text vs pointer for links)
        {
            let ui_weak = ui.as_weak();
            let pdfium = pdfium.clone();
            let state = state.clone();
            let text_cache = text_cache.clone();
            let link_cache = link_cache.clone();
            ui.on_page_hover(move |page, x, y| {
                let ui = ui_weak.unwrap();
                let s = state.lock().unwrap();
                let page_idx = (page - 1) as u16;

                // Ensure link cache is populated
                let mut lcache = link_cache.borrow_mut();
                if !lcache.contains_key(&page_idx) {
                    let page_height_pt = s.page_dims.get(page_idx as usize)
                        .map(|d| if s.rotation == 90 || s.rotation == 270 { d.width_pt } else { d.height_pt })
                        .unwrap_or(841.0);
                    let mut all_links = Vec::new();
                    if let Some(path) = &s.file_path {
                        all_links = links::extract_page_links(&pdfium, path, page_idx, s.scale, page_height_pt);
                    }
                    let mut tcache = text_cache.borrow_mut();
                    ensure_char_cache(&mut tcache, &pdfium, &s, page_idx);
                    if let Some(chars) = tcache.get(&page_idx) {
                        all_links.extend(links::detect_text_urls(chars, s.scale));
                    }
                    lcache.insert(page_idx, all_links);
                }

                // Update link overlay rects for this page (for pointer cursor)
                if ui.get_link_rects_page() != page {
                    if let Some(page_links) = lcache.get(&page_idx) {
                        let rect_model: Vec<TextSelRect> = page_links.iter().map(|l| {
                            TextSelRect { x: l.left, y: l.top, w: l.right - l.left, h: l.bottom - l.top }
                        }).collect();
                        ui.set_link_rects(ModelRc::new(VecModel::from(rect_model)));
                        ui.set_link_rects_page(page);
                    }
                }
            });
        }

        // Handle click on link
        {
            let ui_weak = ui.as_weak();
            let link_cache = link_cache.clone();
            ui.on_open_link(move |page, x, y| {
                let _ui = ui_weak.unwrap();
                let page_idx = (page - 1) as u16;
                let lcache = link_cache.borrow();
                if let Some(page_links) = lcache.get(&page_idx) {
                    if let Some(link) = links::link_at(page_links, x, y) {
                        let _ = open::that(&link.url); // open in default browser
                    }
                }
            });
        }

        {
            let selected_text = selected_text.clone();
            ui.on_copy_selection(move || {
                let text = selected_text.borrow();
                if !text.is_empty() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text.as_str());
                    }
                }
            });
        }
    }

    // ── Scroll position tracking + lazy rendering (poll every 150ms) ────────
    {
        let ui_weak = ui.as_weak();
        let pdfium = pdfium.clone();
        let state = state.clone();
        let pages_vm = pages_vm.clone();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, std::time::Duration::from_millis(150), move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let s = state.lock().unwrap();
            if s.page_count == 0 { return; }
            let scroll_y = ui.get_current_scroll_y();
            let page = page_at_scroll_y(scroll_y, &s);
            if page != ui.get_current_page() {
                ui.set_current_page(page);
                ui.set_page_text(page.to_string().into());
            }

            // Lazy render pages that became visible
            if let Some(vm) = pages_vm.borrow().as_ref() {
                let viewport_h = ui.get_viewer_height();
                render_visible_pages(&pdfium, &s, vm, scroll_y, viewport_h);
            }
        });
        std::mem::forget(timer);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn page_at_scroll_y(scroll_y: f32, state: &ViewerState) -> i32 {
    let gap = 8.0_f32;
    let mut y = 0.0_f32;
    for (i, dim) in state.page_dims.iter().enumerate() {
        let h = if state.rotation == 90 || state.rotation == 270 {
            dim.width_pt
        } else {
            dim.height_pt
        };
        let page_h = h * state.scale;
        if scroll_y < y + page_h * 0.5 {
            return (i + 1) as i32;
        }
        y += page_h + gap;
    }
    state.page_count as i32
}

fn scroll_to_page(ui: &App, state: &ViewerState, page: i32) {
    let page_idx = (page - 1).max(0) as usize;
    let gap = 8.0_f32; // matches spacing in .slint
    let mut y = 0.0_f32;
    for i in 0..page_idx.min(state.page_dims.len()) {
        let dim = &state.page_dims[i];
        let h = if state.rotation == 90 || state.rotation == 270 {
            dim.width_pt
        } else {
            dim.height_pt
        };
        y += h * state.scale + gap;
    }
    // viewport-y is negative (scrolling down = negative offset)
    ui.invoke_scroll_to(-y);
}

struct LoadedDoc {
    dims: Vec<PageDim>,
    annotations: Vec<crate::pdf::models::Annotation>,
}

fn load_document(pdfium: &Pdfium, path: &std::path::Path) -> Result<LoadedDoc, String> {
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("{}", e))?;

    let pages = doc.pages();
    let count = pages.len();
    let mut dims = Vec::with_capacity(count as usize);

    for i in 0..count {
        let page = pages.get(i).map_err(|e| format!("{}", e))?;
        dims.push(PageDim {
            width_pt: page.width().value,
            height_pt: page.height().value,
        });
    }

    // Load annotations from PDF metadata (CCAnnot)
    let lopdf_doc = lopdf::Document::load(path).unwrap_or_default();
    let meta = writer::load_meta(&lopdf_doc);

    Ok(LoadedDoc {
        dims,
        annotations: meta.annotations,
    })
}

/// Hit test: find annotation at canvas pixel (x, y).
/// Returns (index, (canvas_left, canvas_top, canvas_width, canvas_height)) or None.
fn hit_test(
    annotations: &[crate::pdf::models::Annotation],
    canvas_x: f32,
    canvas_y: f32,
    scale: f64,
    page_height_pt: f64,
) -> Option<(usize, (f32, f32, f32, f32))> {
    use crate::pdf::models::Annotation;

    // Test in reverse order (topmost first)
    for (i, ann) in annotations.iter().enumerate().rev() {
        let bounds = ann_canvas_bounds(ann, scale, page_height_pt);
        let (left, top, w, h) = bounds;
        let tolerance = 5.0;
        if canvas_x >= left - tolerance
            && canvas_x <= left + w + tolerance
            && canvas_y >= top - tolerance
            && canvas_y <= top + h + tolerance
        {
            return Some((i, bounds));
        }
    }
    None
}

/// Get canvas-pixel bounding box for an annotation.
fn ann_canvas_bounds(
    ann: &crate::pdf::models::Annotation,
    scale: f64,
    page_height_pt: f64,
) -> (f32, f32, f32, f32) {
    use crate::pdf::models::Annotation;

    match ann {
        Annotation::Rect(r) => {
            let left = (r.x * scale) as f32;
            let top = ((page_height_pt - r.y - r.height) * scale) as f32;
            let w = (r.width * scale) as f32;
            let h = (r.height * scale) as f32;
            (left, top, w, h)
        }
        Annotation::Circle(c) => {
            let left = (c.x * scale) as f32;
            let top = ((page_height_pt - c.y - c.height) * scale) as f32;
            let w = (c.width * scale) as f32;
            let h = (c.height * scale) as f32;
            (left, top, w, h)
        }
        Annotation::Text(t) => {
            let left = (t.x * scale) as f32;
            let top = ((page_height_pt - t.y) * scale) as f32;
            let w = (t.width * scale) as f32;
            let h = (t.font_size * 1.2 * t.content.split('\n').count() as f64 * scale) as f32;
            (left, top, w, h)
        }
        Annotation::Signature(s) => {
            let left = (s.x * scale) as f32;
            let top = ((page_height_pt - s.y - s.height) * scale) as f32;
            let w = (s.width * scale) as f32;
            let h = (s.height * scale) as f32;
            (left, top, w, h)
        }
    }
}

fn set_ann_color(ann: &mut crate::pdf::models::Annotation, color: &crate::pdf::models::RgbColor) {
    match ann {
        crate::pdf::models::Annotation::Rect(r) => r.color = color.clone(),
        crate::pdf::models::Annotation::Circle(c) => c.color = color.clone(),
        crate::pdf::models::Annotation::Text(t) => t.color = color.clone(),
        crate::pdf::models::Annotation::Signature(_) => {}
    }
}

fn set_ann_stroke_width(ann: &mut crate::pdf::models::Annotation, w: f64) {
    match ann {
        crate::pdf::models::Annotation::Rect(r) => r.stroke_width = w,
        crate::pdf::models::Annotation::Circle(c) => c.stroke_width = w,
        _ => {}
    }
}

fn ann_type_str(ann: &crate::pdf::models::Annotation) -> &'static str {
    match ann {
        crate::pdf::models::Annotation::Rect(_) => "rect",
        crate::pdf::models::Annotation::Circle(_) => "circle",
        crate::pdf::models::Annotation::Text(_) => "text",
        crate::pdf::models::Annotation::Signature(_) => "signature",
    }
}

/// Re-render all strokes as smooth Catmull-Rom curves with uniform width.
fn render_smooth_strokes(pixmap: &mut tiny_skia::Pixmap, strokes: &[Vec<(f32, f32)>]) {
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(tiny_skia::Color::from_rgba8(15, 15, 35, 255));
    paint.anti_alias = true;

    for raw_points in strokes {
        if raw_points.len() < 2 { continue; }

        // 1. Simplify: remove noise with Douglas-Peucker
        let simplified = douglas_peucker(raw_points, 1.0);
        if simplified.len() < 2 { continue; }

        // 2. Smooth positions with moving average (2 passes, window 3)
        let mut pts = simplified;
        for _ in 0..2 {
            pts = smooth_points(&pts, 3);
        }
        if pts.len() < 2 { continue; }

        // 3. Compute per-segment width based on orientation, then smooth
        let n = pts.len();
        let mut raw_widths: Vec<f32> = Vec::with_capacity(n);
        for i in 0..n {
            let (dx, dy) = if i == 0 {
                (pts[1].0 - pts[0].0, pts[1].1 - pts[0].1)
            } else if i == n - 1 {
                (pts[n - 1].0 - pts[n - 2].0, pts[n - 1].1 - pts[n - 2].1)
            } else {
                (pts[i + 1].0 - pts[i - 1].0, pts[i + 1].1 - pts[i - 1].1)
            };
            // vertical (|dy| >> |dx|) → thick, horizontal → thin
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let verticality = (dy / len).abs(); // 0.0 = horizontal, 1.0 = vertical
            let min_w = 1.2_f32;
            let max_w = 7.5_f32;
            raw_widths.push(min_w + verticality * (max_w - min_w));
        }

        // Smooth widths heavily to avoid abrupt transitions
        let mut widths = raw_widths;
        for _ in 0..4 {
            widths = smooth_values_f(&widths, 7);
        }

        // 4. Draw segment by segment with varying width
        for i in 0..n - 1 {
            let p0 = if i > 0 { pts[i - 1] } else { pts[i] };
            let p1 = pts[i];
            let p2 = pts[i + 1];
            let p3 = if i + 2 < n { pts[i + 2] } else { pts[i + 1] };

            let cp1x = p1.0 + (p2.0 - p0.0) / 6.0;
            let cp1y = p1.1 + (p2.1 - p0.1) / 6.0;
            let cp2x = p2.0 - (p3.0 - p1.0) / 6.0;
            let cp2y = p2.1 - (p3.1 - p1.1) / 6.0;

            let w = (widths[i] + widths[i + 1]) * 0.5;

            let stroke = tiny_skia::Stroke {
                width: w,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..tiny_skia::Stroke::default()
            };

            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(p1.0, p1.1);
            pb.cubic_to(cp1x, cp1y, cp2x, cp2y, p2.0, p2.1);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
            }
        }
    }
}

/// Resample a polyline using Catmull-Rom interpolation.
/// `subdivisions` = number of intermediate points between each original pair.
fn catmull_rom_resample(points: &[(f32, f32)], subdivisions: usize) -> Vec<(f32, f32)> {
    let n = points.len();
    if n < 2 { return points.to_vec(); }

    let mut result = Vec::with_capacity(n * (subdivisions + 1));

    for i in 0..n - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < n { points[i + 2] } else { points[i + 1] };

        result.push(p1);

        for s in 1..=subdivisions {
            let t = s as f32 / (subdivisions + 1) as f32;
            let t2 = t * t;
            let t3 = t2 * t;

            let x = 0.5 * ((2.0 * p1.0)
                + (-p0.0 + p2.0) * t
                + (2.0 * p0.0 - 5.0 * p1.0 + 4.0 * p2.0 - p3.0) * t2
                + (-p0.0 + 3.0 * p1.0 - 3.0 * p2.0 + p3.0) * t3);
            let y = 0.5 * ((2.0 * p1.1)
                + (-p0.1 + p2.1) * t
                + (2.0 * p0.1 - 5.0 * p1.1 + 4.0 * p2.1 - p3.1) * t2
                + (-p0.1 + 3.0 * p1.1 - 3.0 * p2.1 + p3.1) * t3);
            result.push((x, y));
        }
    }
    result.push(*points.last().unwrap());
    result
}

/// Douglas-Peucker polyline simplification.
fn douglas_peucker(points: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    let n = points.len();
    if n <= 2 { return points.to_vec(); }

    // Find the point farthest from the line (first, last)
    let (ax, ay) = points[0];
    let (bx, by) = points[n - 1];
    let line_len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(0.001);

    let mut max_dist = 0.0_f32;
    let mut max_idx = 0;
    for i in 1..n - 1 {
        let (px, py) = points[i];
        let dist = ((by - ay) * px - (bx - ax) * py + bx * ay - by * ax).abs() / line_len;
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        let mut left = douglas_peucker(&points[..=max_idx], epsilon);
        let right = douglas_peucker(&points[max_idx..], epsilon);
        left.pop(); // avoid duplicate at split point
        left.extend(right);
        left
    } else {
        vec![points[0], points[n - 1]]
    }
}

/// Smooth point positions with moving average.
fn smooth_points(points: &[(f32, f32)], window: usize) -> Vec<(f32, f32)> {
    let n = points.len();
    if n <= 2 { return points.to_vec(); }
    let half = window / 2;
    let mut result = Vec::with_capacity(n);
    // Keep first and last point fixed
    result.push(points[0]);
    for i in 1..n - 1 {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(n);
        let count = (end - start) as f32;
        let mut sx = 0.0_f32;
        let mut sy = 0.0_f32;
        for j in start..end {
            sx += points[j].0;
            sy += points[j].1;
        }
        result.push((sx / count, sy / count));
    }
    result.push(points[n - 1]);
    result
}

/// Smooth a sequence of float values with moving average, keeping first/last fixed.
fn smooth_values_f(values: &[f32], window: usize) -> Vec<f32> {
    let n = values.len();
    if n <= 2 { return values.to_vec(); }
    let half = window / 2;
    let mut result = Vec::with_capacity(n);
    result.push(values[0]);
    for i in 1..n - 1 {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(n);
        let sum: f32 = values[start..end].iter().sum();
        result.push(sum / (end - start) as f32);
    }
    result.push(values[n - 1]);
    result
}

fn update_sig_image(ui: &App, pixmap: &tiny_skia::Pixmap) {
    let w = pixmap.width();
    let h = pixmap.height();
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    buf.make_mut_bytes().copy_from_slice(pixmap.data());
    ui.set_sig_canvas_image(Image::from_rgba8(buf));
}

/// Read form field values from PDF using lopdf (same lib used for writing).
fn read_lopdf_form_values(path: &std::path::Path) -> std::collections::HashMap<String, String> {
    let mut values = std::collections::HashMap::new();
    let Ok(doc) = lopdf::Document::load(path) else { return values };

    // Get AcroForm fields
    let Ok(root_ref) = doc.trailer.get(b"Root") else { return values };
    let Ok(cat_id) = root_ref.as_reference() else { return values };
    let Ok(cat) = doc.get_object(cat_id) else { return values };
    let Ok(dict) = cat.as_dict() else { return values };
    let Ok(lopdf::Object::Reference(acroform_id)) = dict.get(b"AcroForm") else { return values };
    let Ok(acroform) = doc.get_object(*acroform_id) else { return values };
    let Ok(af_dict) = acroform.as_dict() else { return values };
    let Ok(lopdf::Object::Array(fields_arr)) = af_dict.get(b"Fields") else { return values };

    fn collect_fields(doc: &lopdf::Document, refs: &[lopdf::Object], parent_name: &str, values: &mut std::collections::HashMap<String, String>) {
        for obj_ref in refs {
            let lopdf::Object::Reference(id) = obj_ref else { continue };
            let Ok(obj) = doc.get_object(*id) else { continue };
            let Ok(dict) = obj.as_dict() else { continue };

            let partial: String = dict.get(b"T").ok()
                .and_then(|o| match o {
                    lopdf::Object::String(s, _) => String::from_utf8(s.clone()).ok(),
                    _ => None,
                })
                .unwrap_or_default();

            let full_name = if parent_name.is_empty() {
                partial.clone()
            } else if partial.is_empty() {
                parent_name.to_string()
            } else {
                format!("{}.{}", parent_name, partial)
            };

            // Read value
            if let Ok(v) = dict.get(b"V") {
                let val = match v {
                    lopdf::Object::String(s, _) => String::from_utf8(s.clone()).unwrap_or_default(),
                    lopdf::Object::Name(n) => String::from_utf8(n.clone()).unwrap_or_default(),
                    _ => String::new(),
                };
                if !val.is_empty() && !full_name.is_empty() {
                    values.insert(full_name.clone(), val);
                }
            }

            // Recurse into Kids
            if let Ok(lopdf::Object::Array(kids)) = dict.get(b"Kids") {
                collect_fields(doc, kids, &full_name, values);
            }
        }
    }

    collect_fields(&doc, fields_arr, "", &mut values);
    values
}

fn save_annotated_pdf(
    path: &std::path::Path,
    annotations: &[crate::pdf::models::Annotation],
    form_values: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    use std::collections::HashMap;

    let mut doc = lopdf::Document::load(path).map_err(|e| format!("{}", e))?;

    // Load existing meta to preserve stream IDs
    let mut meta = writer::load_meta(&doc);
    meta.annotations = annotations.to_vec();

    // Group annotations by page
    let mut by_page: HashMap<u32, Vec<&crate::pdf::models::Annotation>> = HashMap::new();
    for ann in annotations {
        by_page.entry(ann.page()).or_default().push(ann);
    }

    // Get page object IDs
    let page_ids: Vec<(u32, lopdf::ObjectId)> = doc
        .get_pages()
        .into_iter()
        .map(|(num, id)| (num, id))
        .collect();

    // Write annotations per page
    for (page_num, page_id) in &page_ids {
        let page_anns: Vec<crate::pdf::models::Annotation> = by_page
            .get(page_num)
            .map(|v| v.iter().map(|a| (*a).clone()).collect())
            .unwrap_or_default();

        let existing_sid = meta.stream_ids.get(page_num).map(|arr| (arr[0], arr[1] as u16));

        match writer::write_annotations_for_page(&mut doc, *page_id, &page_anns, existing_sid)? {
            Some(sid) => {
                meta.stream_ids.insert(*page_num, [sid.0, sid.1 as u32]);
            }
            None => {
                meta.stream_ids.remove(page_num);
            }
        }
    }

    // Save metadata
    writer::save_meta(&mut doc, &meta)?;

    // Write to file
    // Write form field values
    if !form_values.is_empty() {
        let fields: Vec<writer::FormFieldValue> = form_values
            .iter()
            .map(|(name, value)| writer::FormFieldValue {
                name: name.clone(),
                value: value.clone(),
            })
            .collect();
        writer::write_form_fields(&mut doc, &fields)?;
    }

    doc.save(path).map_err(|e| format!("{}", e))?;
    Ok(())
}
