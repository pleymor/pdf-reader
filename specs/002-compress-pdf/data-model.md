# Data Model: PDF Compression

**Feature**: 002-compress-pdf
**Date**: 2026-03-25

## Entities

### CompressionLevel (Rust enum, frontend string)

| Field   | Type   | Values                 | Notes                             |
|---------|--------|------------------------|-----------------------------------|
| variant | enum   | Screen / Ebook / Print | Serialised as "screen"/"ebook"/"print" |

Quality targets:
- Screen → JPEG quality 25 (~60–75% size reduction for image-heavy PDFs)
- Ebook  → JPEG quality 55 (~40–55% size reduction)
- Print  → JPEG quality 80 (~20–35% size reduction)

### CompressResult (Rust struct, returned to frontend)

| Field              | Type | Notes                           |
|--------------------|------|---------------------------------|
| original_bytes     | u64  | File size before compression    |
| compressed_bytes   | u64  | File size after compression     |

### CompressModal (TypeScript class)

State held in memory (not persisted):

| Field          | Type             | Notes                          |
|----------------|------------------|--------------------------------|
| selectedLevel  | string           | "screen" \| "ebook" \| "print" |
| isOpen         | boolean          | Whether the modal is visible   |

## State transitions

```
[PDF loaded]
    → user clicks Compress button
    → CompressModal opens (default level = ebook)
    → user picks level + clicks Apply
    → Save As dialog appears
    → user picks output path
    → Rust compress_pdf runs
    → toast shows result
    → modal closes
```
