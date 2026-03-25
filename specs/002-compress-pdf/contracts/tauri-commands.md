# Tauri Command Contracts: PDF Compression

**Feature**: 002-compress-pdf
**Date**: 2026-03-25

## compress_pdf

**Direction**: Frontend → Rust

### Input
```
{
  inputPath:  string   // absolute path to source PDF
  outputPath: string   // absolute path for compressed output
  level:      string   // "screen" | "ebook" | "print"
}
```

### Output (success)
```
{
  originalBytes:    number   // source file size in bytes
  compressedBytes:  number   // output file size in bytes
}
```

### Output (error)
```
string   // human-readable error message
```

### Behaviour
1. Load the source PDF via lopdf.
2. Iterate all object streams; for each DCTDecode image without an SMask,
   decode the JPEG bytes and re-encode at the level's target quality.
3. Save the modified document to `outputPath`.
4. Read file sizes of both paths from the OS.
5. Return `CompressResult`.

### Side effects
- Writes a new file at `outputPath`.
- Source file is never modified.

---

## TypeScript wrapper signature

```typescript
export interface CompressResult {
  originalBytes: number;
  compressedBytes: number;
}

export async function compressPdf(
  inputPath: string,
  outputPath: string,
  level: "screen" | "ebook" | "print",
): Promise<CompressResult>
```
