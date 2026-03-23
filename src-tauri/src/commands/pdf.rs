use lopdf::Document;

use crate::pdf::{models::Annotation, writer};

/// Returns the total number of pages in the given PDF file.
#[tauri::command]
pub fn get_page_count(file_path: String) -> Result<u32, String> {
    let doc = Document::load(&file_path).map_err(|e| e.to_string())?;
    Ok(doc.get_pages().len() as u32)
}

/// Stores `annotations` as JSON metadata in the PDF catalog and saves the result
/// to `output_path`. Annotations are NOT burned into content streams so they
/// remain fully editable when the file is reopened.
#[tauri::command]
pub fn save_annotated_pdf(
    input_path: String,
    output_path: String,
    annotations: Vec<Annotation>,
) -> Result<(), String> {
    let mut doc = Document::load(&input_path).map_err(|e| e.to_string())?;
    writer::store_annotations(&mut doc, &annotations)?;
    doc.save(&output_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Reads editable annotations from the PDF's `CCAnnot` catalog entry.
/// Returns an empty array if the file has no stored annotations.
#[tauri::command]
pub fn read_annotations(file_path: String) -> Result<Vec<Annotation>, String> {
    let doc = Document::load(&file_path).map_err(|e| e.to_string())?;
    writer::load_annotations(&doc)
}
