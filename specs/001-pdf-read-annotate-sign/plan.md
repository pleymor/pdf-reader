# Implementation Plan: PDF Reader with Annotations & Signing

**Branch**: `001-pdf-read-annotate-sign` | **Date**: 2026-03-22 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-pdf-read-annotate-sign/spec.md`

## Summary

Build a minimal Windows desktop PDF reader using Tauri v2. The app renders PDF
pages via pdf.js (WebView2 frontend), captures user-drawn annotations (rectangles,
circles, styled text) and signatures (freehand canvas draw or image insert) in
a TypeScript overlay layer, then burns them permanently into the PDF using lopdf
(pure Rust — no side-car DLLs) on save.

## Technical Context

**Language/Version**: Rust stable (≥ 1.77) + TypeScript 5.x
**Primary Dependencies**: Tauri v2, pdf.js (pdfjs-dist), lopdf 0.39, image crate
**Storage**: Local file system only (no database, no cloud)
**Testing**: `cargo test` (Rust unit tests for PDF writer)
**Target Platform**: Windows x86_64 (`x86_64-pc-windows-msvc`)
**Project Type**: Desktop application (Tauri)
**Performance Goals**: First page render ≤ 2 s for 100-page PDFs; ≤ 300 MB RAM
**Constraints**: Single `.exe`, no side-car DLLs, no installer
**Scale/Scope**: Single document at a time; up to ~500 pages; ≤ 20 annotations typical

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Notes |
|-----------|-------|-------|
| I. Tauri Desktop-First | ✅ PASS | Tauri v2, target `x86_64-pc-windows-msvc` |
| II. Single Executable Distribution | ✅ PASS | lopdf is pure Rust (no DLL); pdf.js bundled via Vite into dist/; `bundle.targets: []` in tauri.conf.json |
| III. Rust Backend, Web Frontend | ✅ PASS | pdf.js renders in WebView (frontend); all PDF file I/O and writing via lopdf in Rust; `invoke()` crosses boundary |
| IV. Simplicity & YAGNI | ✅ PASS | Vanilla TS (no framework); two crates (lopdf, image); no layers beyond what spec requires |
| V. Test-Driven Development | ✅ PASS | Rust unit tests for writer.rs written before implementation; freehand canvas logic tested via manual quickstart checklist |

*Post-design re-check: All principles still satisfied. No violations.*

## Project Structure

### Documentation (this feature)

```text
specs/001-pdf-read-annotate-sign/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── tauri-commands.md  # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
pdf-reader/
├── package.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.ts               # App entry, DocumentState, event wiring
│   ├── pdf-viewer.ts         # pdf.js wrapper, page rendering, zoom
│   ├── canvas-overlay.ts     # Annotation drawing canvas (stacked above PDF)
│   ├── toolbar.ts            # Tool picker, colour/style controls
│   ├── signature-modal.ts    # Freehand canvas + image-insert modal
│   ├── annotation-store.ts   # In-memory Map<page, Annotation[]>
│   ├── models.ts             # TypeScript types (Annotation, ToolState, etc.)
│   └── tauri-bridge.ts       # invoke() wrappers for all Tauri commands
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/
    │   └── default.json      # Permission grants
    └── src/
        ├── main.rs           # Entry point (do not modify)
        ├── lib.rs            # Command registration
        ├── commands/
        │   ├── mod.rs
        │   ├── dialog.rs     # open_pdf_dialog, save_pdf_dialog
        │   └── pdf.rs        # get_page_count, save_annotated_pdf
        └── pdf/
            ├── mod.rs
            ├── models.rs     # Rust Annotation enum + sub-types
            └── writer.rs     # lopdf writing: rect, circle, text, image
```

**Structure Decision**: Single Tauri project (frontend in `src/`, backend in
`src-tauri/`). No monorepo, no separate packages. Chosen because the app is
single-document, single-window, with clear frontend/backend split enforced by
Tauri's architecture.

## Complexity Tracking

> No constitution violations detected. Table intentionally empty.
