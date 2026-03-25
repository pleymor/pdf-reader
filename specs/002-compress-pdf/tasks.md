# Tasks: PDF Compression

**Feature**: 002-compress-pdf
**Date**: 2026-03-25

## T1 — Create `src-tauri/src/pdf/compress.rs`
- Define `CompressionLevel` enum (Screen/Ebook/Print) with `jpeg_quality()` and `from_str()`
- Implement `compress_images(doc: &mut Document, level: CompressionLevel)`
  - Iterate all `Object::Stream` objects
  - Skip non-Image XObjects
  - Skip non-DCTDecode streams
  - Skip streams with `/SMask` (transparency)
  - Decode JPEG → re-encode at target quality → replace only if smaller

## T2 — Create `src-tauri/src/commands/compress.rs`
- Define `CompressResult { original_bytes: u64, compressed_bytes: u64 }`
- Implement `#[tauri::command] compress_pdf(input_path, output_path, level) -> Result<CompressResult, String>`
  - Load doc → compress images → save → stat both files → return sizes

## T3 — Wire Rust modules
- `src-tauri/src/pdf/mod.rs`: add `pub mod compress;`
- `src-tauri/src/commands/mod.rs`: add `pub mod compress;`
- `src-tauri/src/lib.rs`: add `commands::compress::compress_pdf` to `generate_handler!`

## T4 — Add TypeScript bridge
- `src/tauri-bridge.ts`: add `CompressResult` interface + `compressPdf()` async function

## T5 — Add compression modal
- `index.html`: add `#compress-modal` backdrop + three radio buttons + Apply button
- `src/compress-modal.ts`: `CompressModal` class (open/close, level selection, confirm callback)

## T6 — Wire frontend
- `src/toolbar.ts`: add `{ type: "compress" }` to `ToolbarEvent`; add Compress button
- `src/main.ts`: import `CompressModal`; handle `compress` event (open modal → Save As → compress → toast)

## T7 — Add i18n keys
- `src/i18n.ts`: add `btnCompress`, `ttCompress`, `compressTitle`, `compressClose`,
  `compressScreen`, `compressEbook`, `compressPrint`, `compressApply` to all 20 languages

## Dependencies
- T2 depends on T1
- T3 depends on T1, T2
- T5 depends on T4
- T6 depends on T5
- T7 can run in parallel with T1–T6
