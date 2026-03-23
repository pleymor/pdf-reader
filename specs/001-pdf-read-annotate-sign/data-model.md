# Data Model: PDF Reader with Annotations & Signing

**Branch**: `001-pdf-read-annotate-sign`
**Date**: 2026-03-22

---

## Frontend State (TypeScript)

### DocumentState

Held in a module-level singleton; drives the UI.

```typescript
interface DocumentState {
  filePath: string | null;       // Absolute path of the open PDF
  pageCount: number;             // Total pages (0 until PDF loaded)
  currentPage: number;           // 1-indexed current page
  zoomLevel: number;             // Scale factor (1.0 = 100%)
  isDirty: boolean;              // Unsaved changes exist
  pdfDoc: PDFDocumentProxy | null; // pdf.js document reference
}
```

### Annotation Types

All coordinates are in PDF user space (points, bottom-left origin).
The frontend builds these objects, sends them to Rust on save.

```typescript
interface BaseAnnotation {
  page: number;   // 1-indexed page number
  x: number;      // Left edge in PDF points
  y: number;      // Bottom edge in PDF points (PDF coordinate space)
}

interface RectAnnotation extends BaseAnnotation {
  kind: "rect";
  width: number;  // Width in PDF points
  height: number; // Height in PDF points
  color: RgbColor;
  strokeWidth: number; // Border thickness in points (default: 1.5)
}

interface CircleAnnotation extends BaseAnnotation {
  kind: "circle";
  width: number;  // Bounding box width in PDF points
  height: number; // Bounding box height in PDF points
  color: RgbColor;
  strokeWidth: number;
}

interface TextAnnotation extends BaseAnnotation {
  kind: "text";
  width: number;      // Bounding box width (for alignment)
  content: string;    // Text content (may include newlines)
  color: RgbColor;
  fontSize: number;   // In points
  bold: boolean;
  italic: boolean;
  underline: boolean;
  alignment: "left" | "center" | "right";
}

interface SignatureAnnotation extends BaseAnnotation {
  kind: "signature";
  width: number;      // Width in PDF points
  height: number;     // Height in PDF points
  imageData: string;  // Base64-encoded PNG (with or without data: prefix)
}

type Annotation =
  | RectAnnotation
  | CircleAnnotation
  | TextAnnotation
  | SignatureAnnotation;

interface RgbColor {
  r: number; // 0–255
  g: number; // 0–255
  b: number; // 0–255
}
```

### AnnotationStore

Maps page numbers to the list of annotations placed on that page.

```typescript
type AnnotationStore = Map<number, Annotation[]>;
// Key: 1-indexed page number
// Value: ordered array of annotations (draw order = array order)
```

### ActiveToolState

Tracks what the user is currently doing.

```typescript
type ToolKind =
  | "select"      // No annotation tool active — pan/zoom
  | "rect"
  | "circle"
  | "text"
  | "signature";

interface ActiveToolState {
  tool: ToolKind;
  color: RgbColor;    // Active border/text colour
  fontSize: number;   // Active font size (text tool)
  bold: boolean;
  italic: boolean;
  underline: boolean;
  alignment: "left" | "center" | "right";
  strokeWidth: number;
}
```

---

## Backend Data Types (Rust)

Serialised via serde; must match the TypeScript interfaces above.

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Annotation {
    Rect(RectAnnotation),
    Circle(CircleAnnotation),
    Text(TextAnnotation),
    Signature(SignatureAnnotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RectAnnotation {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: RgbColor,
    pub stroke_width: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CircleAnnotation {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: RgbColor,
    pub stroke_width: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextAnnotation {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub content: String,
    pub color: RgbColor,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub alignment: TextAlignment,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignatureAnnotation {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub image_data: String, // Base64-encoded PNG
}
```

---

## State Transitions

### Document Lifecycle

```
Idle
  │─── open_pdf_dialog() selected ──→ Loading
  │
Loading
  │─── pdf.js loaded ──→ Viewing (isDirty=false)
  │─── error ──────────→ Idle (error shown)
  │
Viewing
  │─── add annotation ──→ Viewing (isDirty=true)
  │─── save_pdf ────────→ Saving
  │─── close/exit ──────→ UnsavedWarning (if isDirty)
  │
Saving
  │─── success ─────────→ Viewing (isDirty=false)
  │─── error ───────────→ Viewing (error shown, isDirty unchanged)
  │
UnsavedWarning
  │─── Save ────────────→ Saving
  │─── Discard ─────────→ Idle
  │─── Cancel ──────────→ Viewing
```

### Signature Canvas Lifecycle

```
Closed
  │─── user clicks signature tool ──→ Open (canvas cleared)
  │
Open (drawing)
  │─── pointer down + move ──→ Open (stroke recorded)
  │─── Clear button ──────────→ Open (canvas reset)
  │─── Place button ──────────→ PlacingSignature (canvas exported as PNG)
  │─── Cancel button ─────────→ Closed
  │
PlacingSignature
  │─── user clicks on PDF page ──→ Closed (SignatureAnnotation added to store)
  │─── Escape key ─────────────→ Closed (no annotation added)
```

---

## Coordinate Conversion

pdf.js renders a PDF page at a given scale onto a canvas. The canvas coordinate
system has its origin at the top-left, with Y increasing downward. PDF user
space has its origin at the bottom-left, with Y increasing upward.

```
// canvas → PDF points
pdf_x = canvas_x / viewport.scale
pdf_y = page_height_pt - (canvas_y / viewport.scale)

// PDF points → canvas
canvas_x = pdf_x * viewport.scale
canvas_y = (page_height_pt - pdf_y) * viewport.scale
```

`page_height_pt` is `viewport.viewBox[3]` from pdf.js (page height in points).
