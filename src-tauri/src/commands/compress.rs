use lopdf::Document;
use serde::Serialize;

use crate::pdf::compress::{self, CompressionLevel};

/// Returned to the frontend after a successful compression.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressResult {
    pub original_bytes: u64,
    pub compressed_bytes: u64,
}

/// Loads `input_path`, re-encodes compressible images at the chosen quality
/// level, saves the result to `output_path`, and returns both file sizes.
///
/// The source file is never modified.
#[tauri::command]
pub fn compress_pdf(
    input_path: String,
    output_path: String,
    level: String,
) -> Result<CompressResult, String> {
    let compression_level = CompressionLevel::from_str(&level)
        .ok_or_else(|| format!("Unknown compression level: {level}"))?;

    let original_bytes = std::fs::metadata(&input_path)
        .map_err(|e| e.to_string())?
        .len();

    let mut doc = Document::load(&input_path).map_err(|e| e.to_string())?;

    compress::flatten_forms(&mut doc);
    compress::compress_images(&mut doc, compression_level);
    compress::compress_streams(&mut doc);
    compress::strip_metadata(&mut doc);
    compress::prune_dead_objects(&mut doc);

    doc.save(&output_path).map_err(|e| e.to_string())?;

    let compressed_bytes = std::fs::metadata(&output_path)
        .map_err(|e| e.to_string())?
        .len();

    Ok(CompressResult {
        original_bytes,
        compressed_bytes,
    })
}
