# Quickstart: PDF Compression

**Feature**: 002-compress-pdf

## Using the feature

1. Open any PDF that contains images (e.g. a scanned document).
2. Click the **Compress…** button in the toolbar (visible whenever a PDF is open).
3. Pick a compression level:
   - **Screen** — smallest file, noticeable quality loss at high zoom
   - **Ebook** — balanced (recommended for sharing)
   - **Print** — good quality, moderate size reduction
4. Click **Compress & Save As…** — a Save As dialog appears.
5. Choose a destination and confirm.
6. A toast notification reports the original and compressed file sizes,
   e.g. *"Compressed: 8.2 MB → 2.1 MB (74% smaller)"*.

## Notes

- The original file is **never modified** — the compressed copy is a new file.
- PDFs with no JPEG images (text-only, vector graphics) will see little or no reduction.
- Annotations and signatures embedded in the source PDF are preserved in the output.

## Development: adding a new compression level

In `src-tauri/src/pdf/compress.rs`, add a variant to `CompressionLevel` and its
`jpeg_quality()` and `from_str()` arms. Update the frontend modal in
`src/compress-modal.ts` to expose the new option.
