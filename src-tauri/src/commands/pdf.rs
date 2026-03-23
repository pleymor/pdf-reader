use std::collections::HashMap;

use lopdf::Document;

use crate::pdf::{models::Annotation, writer};

/// Returns the total number of pages in the given PDF file.
#[tauri::command]
pub fn get_page_count(file_path: String) -> Result<u32, String> {
    let doc = Document::load(&file_path).map_err(|e| e.to_string())?;
    Ok(doc.get_pages().len() as u32)
}

/// Reads `input_path`, burns `annotations` into page content streams,
/// and saves the result to `output_path` (may be the same as `input_path`).
#[tauri::command]
pub fn save_annotated_pdf(
    input_path: String,
    output_path: String,
    annotations: Vec<Annotation>,
) -> Result<(), String> {
    let mut doc = Document::load(&input_path).map_err(|e| e.to_string())?;

    // Group annotations by (1-indexed) page number
    let mut by_page: HashMap<u32, Vec<Annotation>> = HashMap::new();
    for ann in annotations {
        by_page.entry(ann.page()).or_default().push(ann);
    }

    // Get the map of page number → ObjectId
    let pages = doc.get_pages();

    for (page_num, anns) in by_page {
        let page_id = pages
            .get(&page_num)
            .copied()
            .ok_or_else(|| format!("page {page_num} not found in document"))?;

        writer::write_annotations_for_page(&mut doc, page_id, &anns)?;
    }

    doc.save(&output_path).map_err(|e| e.to_string())?;
    Ok(())
}
