# Tauri IPC Command Contracts

**Branch**: `001-pdf-read-annotate-sign`
**Date**: 2026-03-22

These are the Rust backend commands exposed to the TypeScript frontend via
`invoke()`. All types use camelCase in TypeScript and snake_case in Rust.

---

## Command: `open_pdf_dialog`

Opens a native Windows file-picker dialog filtered to PDF files. Returns the
selected file path, or `null` if the user cancelled.

**TypeScript call**:
```typescript
const path = await invoke<string | null>('open_pdf_dialog');
```

**Rust signature**:
```rust
#[tauri::command]
async fn open_pdf_dialog(app: AppHandle) -> Result<Option<String>, String>
```

**Response**:
- `string` — absolute file path of the selected PDF
- `null` — user cancelled the dialog

**Error**: Returns `Err(String)` if the dialog itself fails to open (system error).

---

## Command: `save_pdf_dialog`

Opens a native Windows Save As dialog. Returns the chosen output path or `null`
if cancelled. Does not write any file; only returns the path.

**TypeScript call**:
```typescript
const outputPath = await invoke<string | null>('save_pdf_dialog', {
  currentPath: '/path/to/original.pdf'
});
```

**Rust signature**:
```rust
#[tauri::command]
async fn save_pdf_dialog(
    app: AppHandle,
    current_path: String,
) -> Result<Option<String>, String>
```

**Behaviour**: Pre-fills the dialog filename from `currentPath` basename.

**Response**:
- `string` — absolute path where the PDF should be saved
- `null` — user cancelled

---

## Command: `get_page_count`

Returns the number of pages in the given PDF file.

**TypeScript call**:
```typescript
const count = await invoke<number>('get_page_count', { filePath });
```

**Rust signature**:
```rust
#[tauri::command]
fn get_page_count(file_path: String) -> Result<u32, String>
```

**Response**: `u32` — total page count.

**Error**: `Err(String)` with a human-readable message if the file cannot be
read or is not a valid PDF.

**Note**: pdf.js also provides page count from the loaded document; this command
exists as a validation/fallback and for error checking before the frontend loads.

---

## Command: `save_annotated_pdf`

Reads the source PDF, burns the provided annotations into the page content
streams, and writes the result to the output path.

**TypeScript call**:
```typescript
await invoke<void>('save_annotated_pdf', {
  inputPath: '/path/to/original.pdf',
  outputPath: '/path/to/output.pdf',
  annotations: [
    {
      kind: 'rect',
      page: 1,
      x: 72.0, y: 500.0,
      width: 144.0, height: 72.0,
      color: { r: 255, g: 0, b: 0 },
      strokeWidth: 1.5
    },
    {
      kind: 'text',
      page: 1,
      x: 72.0, y: 450.0,
      width: 200.0,
      content: 'Approved',
      color: { r: 0, g: 0, b: 0 },
      fontSize: 14.0,
      bold: true,
      italic: false,
      underline: false,
      alignment: 'left'
    },
    {
      kind: 'signature',
      page: 2,
      x: 300.0, y: 100.0,
      width: 150.0, height: 60.0,
      imageData: 'iVBORw0KGgo...' // base64 PNG, no data: prefix
    }
  ]
});
```

**Rust signature**:
```rust
#[tauri::command]
async fn save_annotated_pdf(
    input_path: String,
    output_path: String,
    annotations: Vec<Annotation>,  // see data-model.md for Annotation enum
) -> Result<(), String>
```

**Behaviour**:
1. Load `input_path` with lopdf.
2. Group annotations by page number.
3. For each page with annotations, append drawing operators to the page's
   content stream (wrapped in a `q … Q` save/restore block).
4. Write the modified document to `output_path`.
5. `input_path` and `output_path` may be the same (overwrite in place).

**Error**: `Err(String)` with a human-readable message on:
- File not found / unreadable
- Output path not writable
- Invalid annotation data (e.g., base64 decode failure)
- PDF parse error

**Annotation kinds accepted**:

| kind        | Required fields                                                           |
|-------------|---------------------------------------------------------------------------|
| `rect`      | page, x, y, width, height, color, strokeWidth                            |
| `circle`    | page, x, y, width, height, color, strokeWidth                            |
| `text`      | page, x, y, width, content, color, fontSize, bold, italic, underline, alignment |
| `signature` | page, x, y, width, height, imageData (base64 PNG)                        |

---

## Capability Permissions

The following Tauri v2 capability entries are required in
`src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "description": "PDF Reader default permissions",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "dialog:allow-save",
    "fs:allow-read-files",
    "fs:allow-write-files"
  ]
}
```

Asset protocol scope (`tauri.conf.json`) must allow reading arbitrary local
paths so pdf.js can load the opened PDF via `convertFileSrc()`:

```json
{
  "app": {
    "security": {
      "assetProtocol": {
        "enable": true,
        "scope": { "allow": ["**"] }
      }
    }
  }
}
```
