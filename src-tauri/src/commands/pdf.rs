use std::collections::HashMap;

use lopdf::Document;

use crate::pdf::{models::Annotation, writer};

/// Returns the total number of pages in the given PDF file.
#[tauri::command]
pub fn get_page_count(file_path: String) -> Result<u32, String> {
    let doc = Document::load(&file_path).map_err(|e| e.to_string())?;
    Ok(doc.get_pages().len() as u32)
}

/// Burns `annotations` into the PDF content streams (visible in all viewers)
/// and also stores them as editable JSON metadata so they can be re-loaded.
///
/// On re-save, existing annotation streams are updated in-place rather than
/// appended, so repeated saves never accumulate stale burn layers.
#[tauri::command]
pub fn save_annotated_pdf(
    input_path: String,
    output_path: String,
    annotations: Vec<Annotation>,
    rotation_delta: i64,
) -> Result<(), String> {
    let mut doc = Document::load(&input_path).map_err(|e| e.to_string())?;

    // Apply rotation to every page's Rotate entry before burning annotations.
    if rotation_delta != 0 {
        let page_ids: Vec<(u32, u16)> = doc.get_pages().values().copied().collect();
        for page_id in page_ids {
            if let Ok(lopdf::Object::Dictionary(ref mut dict)) = doc.get_object_mut(page_id) {
                let current = dict.get(b"Rotate")
                    .ok()
                    .and_then(|o| o.as_i64().ok())
                    .unwrap_or(0);
                let new_rotate = (current + rotation_delta).rem_euclid(360);
                dict.set(b"Rotate", lopdf::Object::Integer(new_rotate));
            }
        }
    }

    let mut meta = writer::load_meta(&doc);

    // Group annotations by page
    let mut by_page: HashMap<u32, Vec<Annotation>> = HashMap::new();
    for ann in &annotations {
        by_page.entry(ann.page()).or_default().push(ann.clone());
    }

    let pages = doc.get_pages();
    let mut new_stream_ids: HashMap<u32, [u32; 2]> = HashMap::new();

    // Write (or update) annotation streams for each page that has annotations
    for (&page_num, anns) in &by_page {
        let page_id = pages
            .get(&page_num)
            .copied()
            .ok_or_else(|| format!("page {page_num} not found in document"))?;

        let existing = meta.stream_ids.get(&page_num).map(|arr| (arr[0], arr[1] as u16));

        if let Some(sid) = writer::write_annotations_for_page(&mut doc, page_id, anns, existing)? {
            new_stream_ids.insert(page_num, [sid.0, sid.1 as u32]);
        }
    }

    // For pages that previously had annotations but now have none, empty their streams
    for (&page_num, arr) in &meta.stream_ids {
        if !by_page.contains_key(&page_num) {
            let sid = (arr[0], arr[1] as u16);
            if let Ok(obj) = doc.get_object_mut(sid) {
                if let lopdf::Object::Stream(s) = obj {
                    s.content = vec![];
                }
            }
        }
    }

    meta.annotations = annotations;
    meta.stream_ids  = new_stream_ids;
    writer::save_meta(&mut doc, &meta)?;

    doc.save(&output_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Reads editable annotations from the PDF's `CCAnnot` catalog entry.
/// Returns an empty array if the file has no stored annotations.
#[tauri::command]
pub fn read_annotations(file_path: String) -> Result<Vec<Annotation>, String> {
    let doc = Document::load(&file_path).map_err(|e| e.to_string())?;
    Ok(writer::load_meta(&doc).annotations)
}
