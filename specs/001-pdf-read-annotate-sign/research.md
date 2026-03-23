# Research: PDF Reader with Annotations & Signing

**Branch**: `001-pdf-read-annotate-sign`
**Date**: 2026-03-22

---

## Decision 1: PDF Rendering Library

**Decision**: pdf.js (pdfjs-dist npm package) running inside the Tauri WebView2 frontend.

**Rationale**:
- Runs entirely in JavaScript inside WebView2 — no Rust integration needed for
  rendering; no native DLLs; no side-car files.
- Mozilla's production-grade renderer used in Firefox; handles complex PDFs
  reliably.
- Renders to an HTML5 Canvas element, which pairs naturally with the annotation
  canvas overlay (both Canvas elements, one stacked on top of the other).
- WebView2 on Windows 11/10 supports all pdf.js requirements.

**Alternatives considered**:
- `pdfium-render` (Rust, wraps Google PDFium): excellent rendering quality but
  requires bundling `pdfium.dll` (~30 MB) alongside the `.exe`, violating
  Constitution Principle II (Single Executable Distribution — no side-car DLLs).
- Server-side rendering via Rust: page images sent to frontend as base64 PNG;
  adds a round-trip per page and Rust-side decode complexity with no benefit
  given WebView2 can render directly.

---

## Decision 2: PDF Modification / Saving Library

**Decision**: `lopdf` (pure Rust, version 0.39.x).

**Rationale**:
- Pure Rust crate; no native DLLs, no external binaries. Constitution Principle
  II is fully satisfied.
- Capable of reading an existing PDF structure and writing new content stream
  operators (rectangles, circles, text, image XObjects) into existing pages.
- Content-stream drawing (burn-in) produces truly flattened output — annotations
  are indistinguishable from original page content in any PDF viewer, matching
  the spec assumption that annotations are permanently embedded.
- Actively maintained (edition 2024, released 2025).

**Alternatives considered**:
- `pdfium-render` for writing too: requires DLL (rejected, same as above).
- `printpdf`: designed for creating new PDFs, not modifying existing ones;
  limited support for layering onto existing page content streams.
- `pdf-rs`: primarily a reader; annotation writing support is unclear and
  under-documented.

---

## Decision 3: Annotation Rendering Strategy

**Decision**: Burn annotations directly into existing PDF page content streams
using lopdf. No interactive PDF annotation objects (/Annots array) are created.

**Rationale**:
- Spec assumption: "Annotations are embedded permanently... cannot be re-edited
  after saving." Content-stream drawing is the truest implementation.
- Drawing into the content stream uses standard PDF graphics operators:
  - Rectangle: `x y w h re W n` + `S` (stroke)
  - Circle: four cubic Bézier curves approximating an ellipse
  - Text: `BT /FontName size Tf x y Td (text) Tj ET`
  - Image: embed as Form XObject, call with `Do` operator
- All standard PDF viewers render content-stream graphics identically.

**Bold / Italic / Underline for Text**:
- PDF standard fonts include bold/italic variants:
  - Regular: `/Helvetica`
  - Bold: `/Helvetica-Bold`
  - Italic: `/Helvetica-Oblique`
  - Bold+Italic: `/Helvetica-BoldOblique`
- Underline: draw a horizontal line at `y - descent` below each text run using
  the stroke operator; width derived from string width estimate.
- Font families are embedded as standard Type1 fonts (always available in any
  PDF viewer without font embedding).

**Text Alignment**:
- Left: text origin at bounding box left edge.
- Center: text origin offset right by `(box_width - text_width) / 2`.
- Right: text origin offset right by `box_width - text_width`.
- Text width is estimated as `char_count × font_size × 0.5` (Helvetica average
  advance). This is an approximation sufficient for MVP.

---

## Decision 4: Freehand Signature Capture

**Decision**: HTML5 Canvas API in the frontend; exported as PNG (base64 DataURL);
sent to Rust backend for embedding as a page image XObject.

**Rationale**:
- Canvas `getContext('2d')` captures pointer events natively with sub-pixel
  accuracy; no additional library needed.
- PNG export is lossless and preserves the transparent background so only the
  ink strokes are placed on the page.
- Rust backend receives base64 PNG, decodes it with the `image` crate (decode
  PNG → raw RGBA pixels), then writes as a PDF image XObject with alpha masking.
- For insertion of a signature image file (PNG/JPEG): same path — frontend reads
  the file via Tauri API and sends base64 to the backend.

---

## Decision 5: Frontend Framework

**Decision**: Vanilla TypeScript compiled with Vite. No JS framework.

**Rationale**:
- Constitution Principle IV: "No abstractions for hypothetical future
  requirements. No heavy JS framework unless justified."
- The UI has three distinct panels (toolbar, PDF view, annotation controls) and
  a handful of state variables; this does not require a reactive component
  framework.
- Vite provides fast HMR during development and bundles TypeScript + pdf.js
  worker into the `dist/` folder consumed by Tauri.

---

## Decision 6: Coordinate System

**Decision**: All annotation coordinates are stored and transmitted as PDF user
space units (points, 1 pt = 1/72 inch), with origin at bottom-left of the page.

**Rationale**:
- lopdf operates in PDF user space; coordinates must be in points.
- pdf.js viewport provides a `transform` matrix; the frontend converts canvas
  pixel positions to PDF user-space points before sending to Rust.
- Conversion: `pdf_x = canvas_x / scale`, `pdf_y = page_height_pt - canvas_y / scale`
  (Y-axis flip from canvas top-left origin to PDF bottom-left origin).

---

## Decision 7: Tauri Configuration

**Decision**: Tauri v2 with the following constraints enforced in `tauri.conf.json`:
- `bundle.targets`: `[]` (empty — no MSI, no NSIS, raw exe only)
- Asset protocol enabled for loading local PDF files via `convertFileSrc()`
- CSP includes `asset:` and `http://asset.localhost` for pdf.js access to files

**Worker bundling**: pdf.js worker (`pdf.worker.min.mjs`) copied to `dist/` via
Vite config so it is served locally without CDN dependency.
