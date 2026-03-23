import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist";
import { convertFileSrc } from "@tauri-apps/api/core";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export class PdfViewer {
  private pdfDoc: PDFDocumentProxy | null = null;
  private currentPageObj: PDFPageProxy | null = null;
  private _currentPage = 1;
  private _pageCount = 0;
  private _scale = 1.5;

  private canvas: HTMLCanvasElement;
  private onPageChangedCb?: (page: number, total: number) => void;
  private onLoadedCb?: (pageCount: number) => void;

  constructor() {
    this.canvas = document.getElementById("pdf-canvas") as HTMLCanvasElement;
  }

  get currentPage(): number { return this._currentPage; }
  get pageCount(): number { return this._pageCount; }
  get scale(): number { return this._scale; }

  /** Height of the current page in PDF points. */
  get pageHeightPt(): number {
    if (!this.currentPageObj) return 841; // A4 fallback
    return this.currentPageObj.getViewport({ scale: 1 }).viewBox[3];
  }

  /** Width of the rendered canvas in pixels. */
  get canvasWidth(): number { return this.canvas.width; }
  /** Height of the rendered canvas in pixels. */
  get canvasHeight(): number { return this.canvas.height; }

  onPageChanged(cb: (page: number, total: number) => void): void {
    this.onPageChangedCb = cb;
  }

  onLoaded(cb: (pageCount: number) => void): void {
    this.onLoadedCb = cb;
  }

  /** Load a PDF from a local file path. */
  async load(filePath: string): Promise<void> {
    if (this.pdfDoc) {
      await this.pdfDoc.destroy();
      this.pdfDoc = null;
    }

    const url = convertFileSrc(filePath);
    const loadingTask = pdfjs.getDocument({ url });
    this.pdfDoc = await loadingTask.promise;
    this._pageCount = this.pdfDoc.numPages;
    this._currentPage = 1;
    this.onLoadedCb?.(this._pageCount);
    await this.render();
  }

  /** Load a password-protected PDF. */
  async loadWithPassword(filePath: string, password: string): Promise<void> {
    if (this.pdfDoc) {
      await this.pdfDoc.destroy();
      this.pdfDoc = null;
    }

    const url = convertFileSrc(filePath);
    const loadingTask = pdfjs.getDocument({ url, password });
    this.pdfDoc = await loadingTask.promise;
    this._pageCount = this.pdfDoc.numPages;
    this._currentPage = 1;
    this.onLoadedCb?.(this._pageCount);
    await this.render();
  }

  /** Render the current page at the current scale. */
  async render(): Promise<void> {
    if (!this.pdfDoc) return;

    const page = await this.pdfDoc.getPage(this._currentPage);
    this.currentPageObj = page;
    const viewport = page.getViewport({ scale: this._scale });

    this.canvas.width = viewport.width;
    this.canvas.height = viewport.height;
    this.canvas.style.width = `${viewport.width}px`;
    this.canvas.style.height = `${viewport.height}px`;

    const ctx = this.canvas.getContext("2d")!;
    ctx.clearRect(0, 0, viewport.width, viewport.height);

    await page.render({ canvasContext: ctx, viewport }).promise;

    this.onPageChangedCb?.(this._currentPage, this._pageCount);
    this.canvas.dispatchEvent(
      new CustomEvent("page-rendered", {
        bubbles: true,
        detail: { page: this._currentPage, width: viewport.width, height: viewport.height },
      })
    );
  }

  async goToPage(n: number): Promise<void> {
    if (!this.pdfDoc) return;
    this._currentPage = Math.max(1, Math.min(n, this._pageCount));
    await this.render();
  }

  async nextPage(): Promise<void> {
    if (this._currentPage < this._pageCount) {
      await this.goToPage(this._currentPage + 1);
    }
  }

  async prevPage(): Promise<void> {
    if (this._currentPage > 1) {
      await this.goToPage(this._currentPage - 1);
    }
  }

  async adjustZoom(delta: number): Promise<void> {
    this._scale = Math.max(0.5, Math.min(3.0, this._scale + delta));
    await this.render();
    return;
  }

  isLoaded(): boolean {
    return this.pdfDoc !== null;
  }

  async close(): Promise<void> {
    if (this.pdfDoc) {
      await this.pdfDoc.destroy();
      this.pdfDoc = null;
      this._pageCount = 0;
      this._currentPage = 1;
    }
    const ctx = this.canvas.getContext("2d")!;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    this.canvas.width = 0;
    this.canvas.height = 0;
  }
}
