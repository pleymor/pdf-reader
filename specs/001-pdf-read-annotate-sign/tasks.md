---
description: "Task list for PDF Reader with Annotations & Signing"
---

# Tasks: PDF Reader with Annotations & Signing

**Input**: Design documents from `/specs/001-pdf-read-annotate-sign/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅

**Tests**: Rust unit tests are included (mandated by Constitution Principle V — TDD).
Frontend visual tests are not included; use the quickstart.md validation checklist instead.

**Organization**: Tasks are grouped by user story to enable independent
implementation and testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1 / US2 / US3)
- Exact file paths are included in all task descriptions

## Path Conventions

Single Tauri project:
- Frontend: `src/` at repository root
- Backend: `src-tauri/src/` at repository root

---

## Phase 1: Setup (Project Initialization)

**Purpose**: Create the Tauri v2 project scaffold and configure all tooling.

- [x] T001 Initialize Tauri v2 project: create `package.json` (with `@tauri-apps/api`, `pdfjs-dist`, `vite`, `typescript`, `@tauri-apps/cli` as devDep), `src-tauri/Cargo.toml` (tauri 2.x, serde, serde_json), `src-tauri/build.rs` (tauri_build::build()), and `index.html` (root div#app, script type=module src=/src/main.ts)
- [x] T002 [P] Configure `src-tauri/tauri.conf.json`: productName "PDF Reader", identifier "com.pdfreader.app", bundle.targets=[], enable assetProtocol (scope allow "**"), CSP with `asset: http://asset.localhost data: blob:`, window 1280×800 resizable
- [x] T003 [P] Create `src-tauri/capabilities/default.json`: grant `core:default`, `dialog:allow-open`, `dialog:allow-save`, `fs:allow-read-files`, `fs:allow-write-files` for window "main"
- [x] T004 [P] Create `vite.config.ts`: TypeScript plugin, root at repo root, outDir `dist/`, copy `node_modules/pdfjs-dist/build/pdf.worker.min.mjs` to `dist/` in build.rollupOptions.plugins
- [x] T005 [P] Create directory skeleton: `src/styles/`, `src-tauri/src/commands/`, `src-tauri/src/pdf/`, `src-tauri/capabilities/`; add placeholder `mod.rs` files in commands/ and pdf/ directories
- [x] T006 [P] Create `src-tauri/src/commands/mod.rs` exporting `dialog` and `pdf` modules; create `src-tauri/src/pdf/mod.rs` exporting `models` and `writer` modules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, IPC wiring, and shared infrastructure all user stories depend on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T007 Define all TypeScript types in `src/models.ts`: `RgbColor`, `BaseAnnotation`, `RectAnnotation`, `CircleAnnotation`, `TextAnnotation`, `SignatureAnnotation`, `Annotation` (union), `ToolKind`, `ActiveToolState`, `DocumentState` — match exactly the shapes in data-model.md
- [x] T008 Implement `src/tauri-bridge.ts`: typed `invoke()` wrappers for all 4 commands — `openPdfDialog()→Promise<string|null>`, `savePdfDialog(currentPath)→Promise<string|null>`, `getPageCount(filePath)→Promise<number>`, `saveAnnotatedPdf(inputPath, outputPath, annotations)→Promise<void>`
- [x] T009 [P] Define Rust types in `src-tauri/src/pdf/models.rs`: `RgbColor`, `TextAlignment` enum, `RectAnnotation`, `CircleAnnotation`, `TextAnnotation`, `SignatureAnnotation` structs, `Annotation` enum with `#[serde(tag="kind", rename_all="camelCase")]` — derive Debug, Serialize, Deserialize, Clone on all
- [x] T010 [P] Add dependencies to `src-tauri/Cargo.toml`: `lopdf = "0.39"`, `image = { version = "0.25", features = ["png"] }`, `base64 = "0.22"`, `tauri-plugin-dialog = "2"`, `tauri-plugin-fs = "2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`
- [x] T011 Implement `src-tauri/src/commands/dialog.rs`: `open_pdf_dialog` (uses tauri-plugin-dialog FileDialogBuilder, filter "PDF Files" `*.pdf`, returns `Option<String>`); `save_pdf_dialog` (pre-fills filename from basename of current_path, filter same, returns `Option<String>`)
- [x] T012 [P] Implement `get_page_count` in `src-tauri/src/commands/pdf.rs`: use `lopdf::Document::load(&file_path)`, return `doc.get_pages().len() as u32`, map errors to `String`
- [x] T013 Register all commands in `src-tauri/src/lib.rs`: call `tauri::Builder::default()`, register plugins `tauri_plugin_dialog::init()` and `tauri_plugin_fs::init()`, `invoke_handler(tauri::generate_handler![dialog::open_pdf_dialog, dialog::save_pdf_dialog, pdf::get_page_count, pdf::save_annotated_pdf])`
- [x] T014 Implement `src/annotation-store.ts`: `AnnotationStore` class with private `Map<number, Annotation[]>`; methods: `add(annotation)`, `remove(page, index)`, `getForPage(page): Annotation[]`, `getAllGrouped(): Map<number, Annotation[]>`, `clear()`, `isEmpty(): boolean`

**Checkpoint**: Foundation ready — all commands registered, types defined, IPC wired

---

## Phase 3: User Story 1 - Open and Read a PDF (Priority: P1) 🎯 MVP

**Goal**: User opens a local PDF, navigates all pages, and zooms in/out. No annotations yet.

**Independent Test**: Open any PDF → first page renders within 2 s → navigate to page 5 →
zoom in 2× → zoom out → close. Passes without any annotation or signing code present.

### Implementation for User Story 1

- [x] T015 [US1] Implement `src/pdf-viewer.ts`: import `pdfjs-dist`, set `GlobalWorkerOptions.workerSrc = '/pdf.worker.min.mjs'`; export `PdfViewer` class with `loadDocument(assetUrl: string)`, `renderPage(pageNum: number, zoom: number)` (renders to `#pdf-canvas`), `pageCount: number`, `currentPage: number`
- [x] T016 [US1] Implement page navigation in `src/pdf-viewer.ts`: add `goToPage(n: number)`, `nextPage()`, `prevPage()` methods; clamp to valid page range; emit a `'page-changed'` CustomEvent on the canvas element
- [x] T017 [US1] Implement `src/toolbar.ts`: `Toolbar` class rendering Open button (calls `openPdfDialog` → `convertFileSrc` → `loadDocument`), page input `<input type="number">` + Prev/Next `<button>` wired to viewer, Zoom In/Out buttons (step 0.25, min 0.5, max 3.0)
- [x] T018 [P] [US1] Implement `src/main.ts`: instantiate `DocumentState`, `PdfViewer`, `Toolbar`, `AnnotationStore`; handle `beforeunload` event showing native browser confirm when `isDirty`; handle `tauri://close-requested` event with Tauri window close guard when `isDirty`
- [x] T019 [P] [US1] Add `src/styles/app.css`: full-height flex column layout; fixed-height toolbar strip at top; scrollable viewer area below; `#viewer-container` with `position: relative` and `display: inline-block` for overlay stacking

**Checkpoint**: US1 complete — user can open any PDF, navigate pages, zoom in/out

---

## Phase 4: User Story 2 - Annotate a PDF (Priority: P2)

**Goal**: User places rectangles, circles, and styled text on pages, then saves to PDF.

**Independent Test**: Open PDF → add red rectangle on p1 → add blue circle on p2 →
add bold 14pt centred black text on p1 → Save As `output.pdf` → open in browser PDF
viewer → all three annotations visible at correct positions.

### Tests for User Story 2 (Rust — Constitution Principle V requires TDD)

> **Write these tests FIRST and confirm they FAIL before implementing the writer functions**

- [x] T020 [US2] Write failing Rust unit tests in `src-tauri/src/pdf/writer.rs` `#[cfg(test)]` module for `write_rect`: create a minimal lopdf Document with one blank page, call `write_rect`, assert the page content stream contains `re` and `S` operators and the expected colour values
- [x] T021 [P] [US2] Write failing Rust unit tests in `src-tauri/src/pdf/writer.rs` `#[cfg(test)]` module for `write_circle`: assert content stream contains Bézier curve `c` operators and the stroke colour values
- [x] T022 [P] [US2] Write failing Rust unit tests in `src-tauri/src/pdf/writer.rs` `#[cfg(test)]` module for `write_text`: assert content stream contains `BT`, `Tf`, `Tj`, `ET` operators with expected font name and size value

### Implementation for User Story 2

- [x] T023 [US2] Implement `src/canvas-overlay.ts`: `CanvasOverlay` class; create `<canvas>` positioned absolutely over `#pdf-canvas` with `pointer-events: auto`; resize canvas to match PDF canvas dimensions on `renderPage`; capture `mousedown`/`mousemove`/`mouseup` events; dispatch `'annotation-created'` CustomEvent with annotation payload
- [x] T024 [US2] Add coordinate conversion to `src/models.ts`: `canvasToPdfCoords(canvasX, canvasY, scale, pageHeightPt): {x, y}` (flip Y axis: `pdfY = pageHeightPt - canvasY/scale`); `pdfToCanvasCoords(pdfX, pdfY, scale, pageHeightPt): {x, y}` (reverse)
- [x] T025 [US2] Implement rectangle drag-draw in `src/canvas-overlay.ts`: on `mousedown` record start point; on `mousemove` with button held, clear overlay canvas and draw a live preview rectangle; on `mouseup` compute PDF coordinates and emit `RectAnnotation` via `'annotation-created'` event
- [x] T026 [P] [US2] Implement circle drag-draw in `src/canvas-overlay.ts`: same drag pattern as rect; use `ellipse()` canvas API for live preview; emit `CircleAnnotation` on release
- [x] T027 [US2] Implement text-annotation tool in `src/canvas-overlay.ts`: on click in text-tool mode, create and position a `<div contenteditable>` over the click point with active style CSS applied; on `blur` or `Enter`, read content, compute PDF coordinates, emit `TextAnnotation`; remove the div
- [x] T028 [US2] Extend `src/toolbar.ts`: add `<input type="color">` for active colour; `<input type="number" min=8 max=72>` for font size; Bold/Italic/Underline toggle `<button>` elements; Left/Center/Right alignment `<button>` group; expose `getActiveStyle(): ActiveToolState`; wire tool buttons (Rect/Circle/Text) to set `ToolKind`
- [x] T029 [US2] Implement `write_rect` in `src-tauri/src/pdf/writer.rs`: function signature `pub fn write_rect(content: &mut Vec<u8>, ann: &RectAnnotation)`; emit PDF operators `q {r} {g} {b} RG {sw} w {x} {y} {w} {h} re S Q\n` (colours normalised 0–1); verify tests pass
- [x] T030 [P] [US2] Implement `write_circle` in `src-tauri/src/pdf/writer.rs`: function `pub fn write_circle(content: &mut Vec<u8>, ann: &CircleAnnotation)`; use 4-arc Bézier approximation (κ ≈ 0.5523) centred at bbox centre; emit `q {colour} RG {sw} w {m c c c h} S Q\n`; verify tests pass
- [x] T031 [US2] Implement `write_text` in `src-tauri/src/pdf/writer.rs`: function `pub fn write_text(content: &mut Vec<u8>, ann: &TextAnnotation, page_resources: &mut lopdf::Dictionary)`; select font name (`/Helvetica`, `/Helvetica-Bold`, `/Helvetica-Oblique`, `/Helvetica-BoldOblique`); compute x offset for alignment; emit `q BT /FontName size Tf r g b rg x y Td (escaped text) Tj ET Q\n`; if underline emit stroke line at `y - 1`; verify tests pass
- [x] T032 [US2] Implement `save_annotated_pdf` command body in `src-tauri/src/commands/pdf.rs`: `lopdf::Document::load(&input_path)`; group annotations by page; for each page call `writer::write_rect/write_circle/write_text/write_image` accumulating content bytes; append to existing page content stream wrapped in `q … Q`; add /Font resource entries for text annotations; `doc.save(&output_path)`
- [x] T033 [US2] Add Save / Save As flow in `src/main.ts`: Save button in toolbar calls `savePdfDialog` if no output path known, then `saveAnnotatedPdf` with all annotations; on success set `isDirty = false`; on error show alert with error message

**Checkpoint**: US2 complete — rect/circle/text annotations placed and saved permanently

---

## Phase 5: User Story 3 - Sign a PDF (Priority: P3)

**Goal**: User draws or imports a signature and places it on a PDF page, then saves.

**Independent Test**: Open PDF → open Signature modal → draw signature → Place →
click page 1 → Save As `signed.pdf` → open in browser PDF viewer → signature visible
at click position.

### Tests for User Story 3 (Rust — Constitution Principle V)

> **Write this test FIRST and confirm it FAILS before implementing `write_image`**

- [x] T034 [US3] Write failing Rust unit test in `src-tauri/src/pdf/writer.rs` `#[cfg(test)]` for `write_image`: generate a 10×10 solid-colour PNG in memory, base64-encode it, call `write_image`, assert the content stream contains `Do` operator and that a `/XObject` key was added to page resources

### Implementation for User Story 3

- [x] T035 [US3] Implement `src/signature-modal.ts`: `SignatureModal` class; render a modal overlay with a 400×200 `<canvas>` for drawing; capture `pointerdown`/`pointermove`/`pointerup` events to draw smooth strokes using `lineTo`/`stroke` with pressure-aware `lineWidth`; expose `open()`, `close()` methods
- [x] T036 [P] [US3] Add Clear / Place / Cancel buttons to `src/signature-modal.ts`: Clear resets canvas; Cancel closes modal; Place calls `canvas.toDataURL('image/png')`, strips `data:...;base64,` prefix, emits `'signature-ready'` CustomEvent with `{ imageData: string, source: 'drawn' }`
- [x] T037 [US3] Implement PlacingSignature state in `src/main.ts`: on `'signature-ready'` event, set cursor to crosshair; next `click` on `#viewer-container` computes PDF coordinates (default size 150×60 pt), creates `SignatureAnnotation`, adds to store, renders PNG image on overlay canvas at click position, resets cursor
- [x] T038 [P] [US3] Add image-insert button to `src/signature-modal.ts`: button opens file picker (PNG/JPEG via `openPdfDialog` filtered to images or via `<input type="file">`); reads file as DataURL via FileReader; updates modal canvas preview; Place emits `{ imageData, source: 'image' }`
- [x] T039 [US3] Implement `write_image` in `src-tauri/src/pdf/writer.rs`: function `pub fn write_image(content: &mut Vec<u8>, ann: &SignatureAnnotation, page_resources: &mut lopdf::Dictionary, doc: &mut lopdf::Document) -> Result<(), String>`; decode base64 with `base64::decode`; decode PNG with `image::load_from_memory`; convert to RGBA; create `/XObject` stream dict with `/Subtype /Image`, `/Width`, `/Height`, `/ColorSpace /DeviceRGB`, `/BitsPerComponent 8`, `/SMask` for alpha channel; add xobject to doc and page resources; emit `q {w} 0 0 {h} {x} {y} cm /Img{id} Do Q\n`; verify test passes
- [x] T040 [US3] Add Signature button to `src/toolbar.ts`: clicking it instantiates and opens `SignatureModal`; connect `'signature-ready'` event from modal to main.ts PlacingSignature handler

**Checkpoint**: US3 complete — drawn and image signatures placed and saved

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Error handling, edge cases, icons, and final verification.

- [x] T041 [P] Add PDF password handling in `src/pdf-viewer.ts`: catch `PasswordException` from pdf.js `getDocument()` promise; show a `<dialog>` prompt for password; retry `getDocument({ url, password })`; show error and abort on wrong password
- [x] T042 [P] Add general error handling in `src/pdf-viewer.ts` and `src/main.ts`: catch all pdf.js load errors and Tauri command rejections; display a non-blocking toast/alert with the error message; ensure app remains stable (no blank state, previous document re-rendered if applicable)
- [x] T043 Generate app icons: run `cargo tauri icon src-tauri/icons/icon.png` (or place placeholder 32×32 / 128×128 / 256×256 / 512×512 PNGs in `src-tauri/icons/`) and update `tauri.conf.json` `bundle.icon` field
- [x] T044 [P] Run full test suite: `cd src-tauri && cargo test` — all Rust unit tests (T020–T022, T034) must pass; fix any failures before marking this phase complete

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Phase 2 — no dependency on US2 or US3
- **US2 (Phase 4)**: Depends on Phase 2 — no dependency on US1 (can parallelize with US1 if staffed)
- **US3 (Phase 5)**: Depends on Phase 2 — builds on US2 writer.rs; US2 SHOULD complete first
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Starts after Foundational — independent of US2/US3
- **US2 (P2)**: Starts after Foundational — independent of US1 (can parallel)
- **US3 (P3)**: Starts after Foundational — adds to writer.rs started in US2; SHOULD follow US2

### Within Each User Story

- Constitution Principle V: Rust unit tests MUST be written and FAIL before implementations
- Frontend: overlay canvas before tool logic before toolbar wiring
- Backend: types before command stubs before full command body

### Parallel Opportunities

- Phase 1: T002–T006 all parallel after T001
- Phase 2: T009, T010, T012 parallel; T011, T013, T014 after T009/T010
- Phase 3: T018, T019 parallel after T015/T016/T017
- Phase 4 tests: T021, T022 parallel with T020
- Phase 4 impl: T030 parallel with T029; T025, T026 parallel after T023; T028 after T027
- Phase 5: T036, T038 parallel with T035; T040 parallel with T039
- Phase 6: T041, T042, T044 all parallel; T043 independent

---

## Parallel Example: User Story 2

```bash
# Launch all Rust tests together (write first, all must FAIL):
Task: "write_rect unit test in src-tauri/src/pdf/writer.rs"
Task: "write_circle unit test in src-tauri/src/pdf/writer.rs"   # parallel
Task: "write_text unit test in src-tauri/src/pdf/writer.rs"     # parallel

# Then launch writer implementations (make tests pass):
Task: "Implement write_rect"
Task: "Implement write_circle"   # parallel
# write_text after write_rect (shares font logic)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: open several PDFs, navigate, zoom — quickstart.md US1 checklist
5. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → scaffold ready
2. User Story 1 → working PDF viewer (**demo**)
3. User Story 2 → annotations + save (**demo**)
4. User Story 3 → signing (**demo**)
5. Polish → production-ready

### Parallel Team Strategy

With two developers:

1. Both complete Setup + Foundational together
2. Dev A: US1 (viewer, toolbar, navigation)
3. Dev B: US2 Rust writer + tests (writer.rs) in parallel with Dev A
4. Dev A adds annotation overlay UI (US2 frontend) after US1 done
5. Dev A + Dev B integrate US2 (frontend + backend)
6. Both tackle US3 and Polish

---

## Notes

- `[P]` tasks can be started in parallel — they operate on different files
- `[USx]` label traces each task to its user story for independent delivery
- Rust unit tests (T020–T022, T034) MUST fail before their implementations
- Commit after each checkpoint (end of each phase)
- Quickstart.md validation checklist is the acceptance test for each story
- Avoid: cross-story state sharing, hard-coded page sizes (use pdf.js viewport), blocking main thread with PDF I/O
