import { invoke } from "@tauri-apps/api/core";
import type { Annotation } from "./models";

/** Opens a file-picker dialog filtered to PDF files. Returns the path or null. */
export async function openPdfDialog(): Promise<string | null> {
  return invoke<string | null>("open_pdf_dialog");
}

/** Opens a Save As dialog. Returns the chosen path or null. */
export async function savePdfDialog(
  currentPath: string
): Promise<string | null> {
  return invoke<string | null>("save_pdf_dialog", {
    currentPath,
  });
}

/** Returns the number of pages in the PDF at `filePath`. */
export async function getPageCount(filePath: string): Promise<number> {
  return invoke<number>("get_page_count", { filePath });
}

/**
 * Burns `annotations` into the PDF at `inputPath` and writes the result
 * to `outputPath`. They may be the same path (overwrite).
 */
export async function saveAnnotatedPdf(
  inputPath: string,
  outputPath: string,
  annotations: Annotation[]
): Promise<void> {
  return invoke<void>("save_annotated_pdf", {
    inputPath,
    outputPath,
    annotations,
  });
}
