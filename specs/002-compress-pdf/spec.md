# Feature Specification: PDF Compression

**Feature Branch**: `002-compress-pdf`
**Created**: 2026-03-25
**Status**: Draft

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Compress an Open PDF (Priority: P1)

A user has a PDF open in the application and wants to reduce its file size before
sending it by email or uploading it to a service with a file-size limit. They
activate the Compress option, choose a compression level, and receive a smaller
PDF that still looks good enough for its intended purpose.

**Why this priority**: File size reduction is the primary value of this feature.
Everything else depends on being able to produce a smaller output file.

**Independent Test**: Open any multi-page PDF containing images, choose
"Compress" with each available level, save the result, verify the output file is
smaller than the original and opens correctly in a standard PDF viewer.

**Acceptance Scenarios**:

1. **Given** a PDF is open, **When** the user activates the Compress option,
   **Then** a compression panel or dialog appears offering at least two named
   compression levels.
2. **Given** the compression panel is open, **When** the user selects a
   compression level and confirms, **Then** the application processes the file
   and offers a Save As dialog for the compressed output.
3. **Given** the user saves the compressed file, **When** they check the file
   size, **Then** the output is smaller than the original for any PDF that
   contains compressible content (images, redundant data).
4. **Given** a compressed PDF is saved, **When** the user opens it in any
   standard PDF viewer, **Then** all pages display without errors or visible
   data loss at the chosen quality level.
5. **Given** a compression level labelled "High Quality" (or equivalent),
   **When** the user compresses and inspects the result, **Then** text remains
   sharp and images are still clearly legible.

---

### User Story 2 - Preview Compression Results Before Saving (Priority: P2)

Before committing to saving, the user can see an estimate of the compressed file
size and a quality indicator so they can decide whether the trade-off is
acceptable.

**Why this priority**: Informed decisions prevent frustration from saving a file
that turns out unusable or barely smaller.

**Independent Test**: Open a large PDF with images, open the compression panel,
observe the estimated output size for each level without saving, switch levels
and confirm the estimate changes, then decide whether to proceed.

**Acceptance Scenarios**:

1. **Given** the compression panel is open, **When** the user selects a
   compression level, **Then** an estimated output file size (e.g., "~2.1 MB")
   is shown alongside the original size.
2. **Given** the compression panel shows estimates, **When** the user switches
   between levels, **Then** the estimated size updates to reflect the new level.
3. **Given** the panel shows estimates, **When** the user decides not to
   compress, **Then** they can dismiss the panel without any file being written.

---

### User Story 3 - Compress on Save (Priority: P3)

A user who has annotated a document can choose to apply compression at save time
so they get a single, smaller annotated PDF without an extra step.

**Why this priority**: Integrating compression into the existing save workflow
reduces friction for users who always want smaller output.

**Acceptance Scenarios**:

1. **Given** a user is saving (Save As), **When** a "Compress output" checkbox
   is available and checked, **Then** the saved file has compression applied at
   the last-used compression level.
2. **Given** a user saves with compression enabled, **When** they check the
   resulting file, **Then** the file is smaller than an equivalent uncompressed
   save while retaining all annotations.

---

### Edge Cases

- What happens when the PDF has no compressible content (text-only, already
  optimised)?
  The application informs the user that no significant size reduction is possible
  and the estimated output size equals or exceeds the original; the user may
  still proceed.
- What happens when the original PDF is already the minimum achievable size?
  The output file may be the same size or marginally larger due to overhead;
  the user is warned before saving.
- What happens when the source PDF is password-protected?
  The user must unlock the PDF before compression can be applied; the compressed
  output inherits the original password or the password is removed based on
  user preference.
- What happens when compression fails (corrupt source, out-of-disk-space)?
  An error message is shown; no partial file is written; the original file is
  unchanged.
- What happens with very large PDFs (hundreds of pages, 500 MB+)?
  A progress indicator is shown; the UI remains responsive; the user can cancel
  the operation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST provide a Compress option accessible from
  the toolbar or main menu when a PDF is open.
- **FR-002**: The compression feature MUST offer at least three named quality
  levels (e.g., Screen / Ebook / Print or Low / Medium / High Quality) that
  correspond to meaningfully different output sizes and visual qualities.
- **FR-003**: The compression panel MUST display the original file size and an
  estimated compressed file size for the selected level before the user commits
  to saving.
- **FR-004**: After choosing a level and confirming, the application MUST write
  a compressed copy of the document via a Save As dialog; the original file MUST
  NOT be modified.
- **FR-005**: The compressed output MUST be a valid PDF that opens correctly in
  mainstream PDF viewers.
- **FR-006**: All existing annotations and signatures embedded in the source PDF
  MUST be preserved in the compressed output.
- **FR-007**: The application MUST display a progress indicator for compression
  operations that take more than one second.
- **FR-008**: The user MUST be able to cancel an in-progress compression
  operation; cancellation MUST leave the original file unchanged.
- **FR-009**: If the source PDF cannot be meaningfully reduced in size, the
  application MUST notify the user before they save.
- **FR-010**: The "Compress output" option MUST be available as a checkbox in
  the Save As workflow so users can combine saving and compressing in one step.

### Key Entities

- **CompressionLevel**: A named preset — name (e.g., "Screen", "Ebook",
  "Print"), target image DPI, image quality setting, and whether to remove
  embedded metadata.
- **CompressionJob**: A transient record of a single compression run — source
  file path, selected level, original size, estimated output size, actual output
  size, status (pending / running / done / cancelled / failed), progress (0–100%).
- **CompressedOutput**: The resulting file — output path, actual size, whether
  it replaced or accompanied the original.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a typical 10 MB scanned PDF (image-heavy), the "Screen" or
  lowest-quality level produces an output at least 60% smaller than the
  original.
- **SC-002**: For the same file, the highest-quality level produces an output
  where image quality is indistinguishable from the original at normal reading
  zoom (100%).
- **SC-003**: Compression of a 50-page, 20 MB PDF completes within 30 seconds
  on a typical consumer laptop.
- **SC-004**: A user can select a compression level and initiate compression
  within 3 clicks or interactions from the main toolbar.
- **SC-005**: All text content in the compressed output remains fully selectable
  and searchable (no rasterisation of text-only pages).
- **SC-006**: The application remains responsive (no UI freeze) during
  compression of any file up to 200 MB.

## Assumptions

- Compression is achieved primarily by downsampling and re-encoding embedded
  images; pure-text PDFs will see little or no size reduction.
- The compressed file is always saved as a new file (Save As); in-place
  overwrite of the source file is not the default to protect the original.
- Preset levels map to specific image resolution and quality targets (e.g.,
  Screen ≈ 72 DPI, Ebook ≈ 150 DPI, Print ≈ 300 DPI) — exact values are an
  implementation detail outside this spec.
- Embedded fonts are subsetted but not removed; font subsetting reduces size
  without affecting readability.
- The feature operates on the currently open (and unlocked) file only; batch
  compression of multiple files is out of scope for this version.
- Metadata removal (author, creator, timestamps) is treated as an optional
  setting within a preset, not a separate feature.
