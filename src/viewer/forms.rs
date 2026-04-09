use pdfium_render::prelude::*;

/// Type of form field.
#[derive(Clone, Debug, PartialEq)]
pub enum FormFieldType {
    Text,
    CheckBox,
    Radio,
    Dropdown,
}

/// A form field with its position and value.
#[derive(Clone, Debug)]
pub struct FormField {
    pub name: String,
    pub field_type: FormFieldType,
    pub page: u16,
    /// Canvas pixel coordinates
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    /// Current value
    pub value: String,
    /// Checked state for checkboxes
    pub checked: bool,
    pub multiline: bool,
}

/// Extract form fields from a PDF page.
pub fn extract_form_fields(
    pdfium: &Pdfium,
    path: &std::path::Path,
    page_index: u16,
    scale: f32,
    page_height_pt: f32,
) -> Vec<FormField> {
    let Ok(doc) = pdfium.load_pdf_from_file(path, None) else { return Vec::new() };
    let Ok(page) = doc.pages().get(page_index) else { return Vec::new() };

    let mut fields = Vec::new();

    for ann in page.annotations().iter() {
        if ann.annotation_type() != PdfPageAnnotationType::Widget { continue; }

        let Some(widget) = ann.as_widget_annotation() else { continue };
        let Some(form_field) = widget.form_field() else { continue };

        let field_name = form_field.name().unwrap_or_default();
        if field_name.is_empty() { continue; }

        let Ok(rect) = ann.bounds() else { continue };

        let left = rect.left.value * scale;
        let top_px = (page_height_pt - rect.top.value) * scale;
        let right = rect.right.value * scale;
        let bottom_px = (page_height_pt - rect.bottom.value) * scale;

        let field_left = left;
        let field_top = top_px.min(bottom_px);
        let field_width = (right - left).abs();
        let field_height = (bottom_px - top_px).abs();

        if field_width < 2.0 || field_height < 2.0 { continue; }

        match form_field {
            PdfFormField::Text(text_field) => {
                let value = text_field.value().unwrap_or_default();
                let multiline = text_field.is_multiline();
                fields.push(FormField {
                    name: field_name,
                    field_type: FormFieldType::Text,
                    page: page_index,
                    left: field_left, top: field_top,
                    width: field_width, height: field_height,
                    value,
                    checked: false,
                    multiline,
                });
            }
            PdfFormField::Checkbox(cb_field) => {
                let checked = cb_field.is_checked().unwrap_or(false);
                fields.push(FormField {
                    name: field_name,
                    field_type: FormFieldType::CheckBox,
                    page: page_index,
                    left: field_left, top: field_top,
                    width: field_width, height: field_height,
                    value: if checked { "true".into() } else { "false".into() },
                    checked,
                    multiline: false,
                });
            }
            PdfFormField::RadioButton(_rb_field) => {
                fields.push(FormField {
                    name: field_name,
                    field_type: FormFieldType::Radio,
                    page: page_index,
                    left: field_left, top: field_top,
                    width: field_width, height: field_height,
                    value: String::new(),
                    checked: false,
                    multiline: false,
                });
            }
            PdfFormField::ComboBox(_) | PdfFormField::ListBox(_) => {
                fields.push(FormField {
                    name: field_name,
                    field_type: FormFieldType::Dropdown,
                    page: page_index,
                    left: field_left, top: field_top,
                    width: field_width, height: field_height,
                    value: String::new(),
                    checked: false,
                    multiline: false,
                });
            }
            _ => {}
        }
    }

    fields
}
