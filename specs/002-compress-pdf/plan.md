# Implementation Plan: PDF Compression

**Feature**: 002-compress-pdf
**Date**: 2026-03-25

## Technical Context

- **Stack**: Rust (lopdf 0.39, image 0.25) + TypeScript 5.x (Tauri v2)
- **Compression target**: DCTDecode (JPEG) image XObjects in PDF files
- **UI pattern**: Modal (same pattern as signature/settings modals)
- **Save pattern**: Always Save As — never overwrites the source

## Phase 1: Rust backend

### New files
| File | Purpose |
|------|---------|
| `src-tauri/src/pdf/compress.rs` | Core image re-encoding logic |
| `src-tauri/src/commands/compress.rs` | Tauri command `compress_pdf` |

### Modified files
| File | Change |
|------|--------|
| `src-tauri/src/pdf/mod.rs` | `pub mod compress;` |
| `src-tauri/src/commands/mod.rs` | `pub mod compress;` |
| `src-tauri/src/lib.rs` | Register `commands::compress::compress_pdf` |

## Phase 2: TypeScript frontend

### New files
| File | Purpose |
|------|---------|
| `src/compress-modal.ts` | `CompressModal` class |

### Modified files
| File | Change |
|------|--------|
| `src/tauri-bridge.ts` | Add `compressPdf()` wrapper + `CompressResult` type |
| `src/toolbar.ts` | Add `compress` event type + Compress button |
| `src/main.ts` | Import `CompressModal`; handle `compress` event |
| `src/i18n.ts` | Add 8 translation keys × 20 languages |
| `index.html` | Add `#compress-modal` HTML element |

## Architecture notes

- The Rust command reads the input file, processes it in memory, writes a new file,
  then returns both file sizes using `std::fs::metadata`.
- The modal is purely presentational — no state persistence needed.
- The compress button lives in `documentSection` (always visible when PDF loaded),
  not inside `annotationSection`.
- Toast format: `"Compressed: X MB → Y MB (Z% smaller)"`.
