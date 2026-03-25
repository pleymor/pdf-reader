# Research: PDF Compression

**Feature**: 002-compress-pdf
**Date**: 2026-03-25

## Decision 1: Compression mechanism

**Decision**: Re-encode DCTDecode (JPEG) image streams at lower JPEG quality.

**Rationale**: PDF files that benefit most from compression are scanned documents and
photo-heavy PDFs, where most of the file size is JPEG image data. Re-encoding at
lower quality is straightforward with the `image` crate already in Cargo.toml.
lopdf 0.39 stores DCTDecode stream content as raw JPEG bytes, so no intermediate
decompression step is needed.

**Alternatives considered**:
- Full ghostscript pipeline: too heavy a dependency for a portable app.
- Downsampling by DPI: requires knowing placement transform of each image on each
  page — complex and fragile. Not needed for v1.
- FlateDecode image conversion to JPEG: would lose alpha transparency for signature
  images. Skipped to avoid data corruption.

## Decision 2: Compression levels

**Decision**: Three named presets — Screen (JPEG q=25), Ebook (JPEG q=55), Print (JPEG q=80).

**Rationale**: Matches the well-known Ghostscript PDF distiller presets familiar to
ilovepdf users. Screen targets maximum reduction; Ebook balances quality and size;
Print preserves good reproduction quality.

**Alternatives considered**: Two levels (low/high) — rejected as too coarse.

## Decision 3: Safety rules for image skipping

**Decision**: Skip any image XObject that:
- Does not use DCTDecode filter (i.e., FlateDecode raw pixels — likely signatures with alpha)
- Has a `/SMask` entry (alpha transparency — must not be converted to lossy JPEG)
- Produces a larger output than the original after re-encoding (no regression)

**Rationale**: Prevents corruption of transparency data in placed signatures.

## Decision 4: UI integration point

**Decision**: Toolbar button "Compress…" visible whenever a PDF is loaded (outside
annotation mode, next to Save As). Opens a modal to pick a level, then triggers
a Save As dialog and reports original vs. compressed size in a toast.

**Rationale**: Mirrors ilovepdf's flow. Does not disrupt the existing annotate/save
workflow. No changes to the save pipeline needed.

## Decision 5: Output size reporting

**Decision**: After compression, return `{ original_bytes, compressed_bytes }` from
the Rust command and show a toast like "Compressed: 8.2 MB → 2.1 MB (74% smaller)".

**Rationale**: Concrete numbers give users confidence the operation worked without
needing an explicit preview step.
