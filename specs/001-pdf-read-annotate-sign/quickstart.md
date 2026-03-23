# Quickstart: PDF Reader Development Guide

**Branch**: `001-pdf-read-annotate-sign`
**Date**: 2026-03-22

---

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable (≥ 1.77) | https://rustup.rs |
| Node.js | ≥ 20 LTS | https://nodejs.org |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2"` |
| WebView2 | pre-installed | Ships with Windows 10 (1803+) and Windows 11 |

Verify:
```bash
rustc --version          # rust 1.77+
node --version           # v20+
cargo tauri --version    # tauri-cli 2.x
```

---

## Project Setup (first time)

```bash
# Install frontend dependencies
npm install

# Install Rust dependencies (auto-happens on first build/dev)
cd src-tauri && cargo fetch && cd ..
```

---

## Development

```bash
cargo tauri dev
```

This starts the Vite dev server (port 5173) and launches the Tauri window.
Hot-reload is active for frontend changes. Rust changes trigger a recompile.

---

## Building for Release

```bash
cargo tauri build --target x86_64-pc-windows-msvc
```

The `.exe` is produced at:
```
src-tauri/target/x86_64-pc-windows-msvc/release/pdf-reader.exe
```

No installer is created — only the raw executable. The file is self-contained
(no side-car DLLs or data directories required).

---

## Running Tests

```bash
# Rust unit tests (PDF writing logic)
cd src-tauri && cargo test

# All tests with output
cd src-tauri && cargo test -- --nocapture
```

---

## Validation Checklist

After building, verify the following manually:

### PDF Viewing (US1)
- [ ] Launch `pdf-reader.exe`
- [ ] Click Open — file picker appears, filtered to PDF
- [ ] Open a 10+ page PDF — first page renders within 2 seconds
- [ ] Navigate next/previous pages — page counter updates correctly
- [ ] Enter a specific page number — jumps to that page
- [ ] Zoom in and out — text and images scale proportionally
- [ ] Open a password-protected PDF — password prompt appears

### Annotations (US2)
- [ ] Select rectangle tool, drag on page — rectangle appears with chosen colour
- [ ] Select circle tool, drag on page — circle appears with chosen colour
- [ ] Select text tool, click on page, type — text appears with chosen style
- [ ] Change font size, bold, italic, underline, alignment — text updates
- [ ] Save As → open output in Adobe Reader or browser — all annotations visible
  at correct positions

### Signing (US3)
- [ ] Open signature modal — blank canvas appears
- [ ] Draw signature with mouse — smooth strokes appear
- [ ] Click Clear — canvas resets
- [ ] Click Place, click on PDF page — signature placed at click position
- [ ] Insert image signature — file picker for PNG/JPEG; image placed on page
- [ ] Save — signature visible in output file

### Edge Cases
- [ ] Close with unsaved changes — warning dialog appears with Save/Discard
- [ ] Open corrupted PDF — error message shown, app remains stable
- [ ] Very large PDF (100+ pages) — memory stays under 300 MB (check Task Manager)

---

## Project Structure

```
pdf-reader/
├── package.json
├── vite.config.ts
├── index.html
├── src/                          # TypeScript frontend
│   ├── main.ts                   # App entry point, DocumentState
│   ├── pdf-viewer.ts             # pdf.js integration, page rendering
│   ├── canvas-overlay.ts         # Annotation drawing canvas
│   ├── toolbar.ts                # Tool selection, style controls
│   ├── signature-modal.ts        # Freehand signature canvas + image insert
│   ├── annotation-store.ts       # AnnotationStore (Map<page, Annotation[]>)
│   ├── models.ts                 # TypeScript type definitions
│   └── tauri-bridge.ts           # invoke() wrappers for all Tauri commands
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json          # Permission grants
│   └── src/
│       ├── main.rs               # Tauri entry point (do not modify)
│       ├── lib.rs                # Command registration
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── dialog.rs         # open_pdf_dialog, save_pdf_dialog
│       │   └── pdf.rs            # get_page_count, save_annotated_pdf
│       └── pdf/
│           ├── mod.rs
│           ├── models.rs         # Annotation, RgbColor, etc. (Rust types)
│           └── writer.rs         # lopdf-based PDF writing logic
└── specs/
    └── 001-pdf-read-annotate-sign/
        ├── spec.md
        ├── plan.md
        ├── research.md
        ├── data-model.md
        ├── quickstart.md        # This file
        ├── contracts/
        │   └── tauri-commands.md
        └── tasks.md
```
