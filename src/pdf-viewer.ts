import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy, PDFPageProxy, PageViewport } from "pdfjs-dist";
import { convertFileSrc } from "@tauri-apps/api/core";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export class PdfViewer {
  private pdfDoc: PDFDocumentProxy | null = null;
  private currentPageObj: PDFPageProxy | null = null;
  private _currentPage = 1;
  private _pageCount = 0;
  private _scale = 1.5;
  private _rotation = 0;
  private _viewport: PageViewport | null = null;

  private canvas: HTMLCanvasElement;
  private textLayerDiv: HTMLElement;
  private onPageChangedCb?: (page: number, total: number) => void;
  private onLoadedCb?: (pageCount: number) => void;

  constructor() {
    this.canvas = document.getElementById("pdf-canvas") as HTMLCanvasElement;
    this.textLayerDiv = document.getElementById("text-layer") as HTMLElement;
  }

  get currentPage(): number { return this._currentPage; }
  get pageCount(): number { return this._pageCount; }
  get scale(): number { return this._scale; }
  get rotation(): number { return this._rotation; }

  /** The pdfjs PageViewport for the current render — includes rotation and scale. */
  get currentViewport(): PageViewport | null { return this._viewport; }

  /** Height of the current page in PDF points (accounts for rotation). */
  get pageHeightPt(): number {
    if (this._viewport) return this._viewport.height / this._scale;
    if (!this.currentPageObj) return 841; // A4 fallback
    return this.currentPageObj.getViewport({ scale: 1, rotation: this._rotation }).viewBox[3];
  }

  /** Width of the rendered canvas in pixels. */
  get canvasWidth(): number { return this.canvas.width; }
  /** Height of the rendered canvas in pixels. */
  get canvasHeight(): number { return this.canvas.height; }

  /** Width of the current page in PDF points (accounts for rotation). */
  get pageWidthPt(): number {
    if (this._viewport) return this._viewport.width / this._scale;
    if (!this.currentPageObj) return 595; // A4 fallback
    return this.currentPageObj.getViewport({ scale: 1, rotation: this._rotation }).viewBox[2];
  }

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
    this._rotation = 0;
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
    this._rotation = 0;
    this.onLoadedCb?.(this._pageCount);
    await this.render();
  }

  /** Rotate the document view 90° clockwise and re-render. Marks the document dirty. */
  async rotate(): Promise<void> {
    this._rotation = (this._rotation + 90) % 360;
    await this.render();
  }

  /** Render the current page at the current scale. */
  async render(): Promise<void> {
    if (!this.pdfDoc) return;

    const page = await this.pdfDoc.getPage(this._currentPage);
    this.currentPageObj = page;
    const viewport = page.getViewport({ scale: this._scale, rotation: (page.rotate + this._rotation) % 360 });
    this._viewport = viewport;

    this.canvas.width = viewport.width;
    this.canvas.height = viewport.height;
    this.canvas.style.width = `${viewport.width}px`;
    this.canvas.style.height = `${viewport.height}px`;

    const ctx = this.canvas.getContext("2d")!;
    ctx.clearRect(0, 0, viewport.width, viewport.height);

    await page.render({ canvasContext: ctx, viewport }).promise;

    // Render selectable text layer
    void this.buildTextLayer(page, viewport);

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

  async setScale(scale: number): Promise<void> {
    this._scale = Math.max(0.25, Math.min(5.0, scale));
    await this.render();
  }

  /** Build a transparent, selectable text layer from raw text content.
   *  Each text item becomes an absolutely-positioned span with inline styles only,
   *  so no external CSS file is required. */
  private async buildTextLayer(page: PDFPageProxy, viewport: PageViewport): Promise<void> {
    this.textLayerDiv.innerHTML = "";
    this.textLayerDiv.style.width  = `${viewport.width}px`;
    this.textLayerDiv.style.height = `${viewport.height}px`;

    const textContent = await page.getTextContent();
    const fragment = document.createDocumentFragment();

    // Off-screen canvas for measuring rendered text widths
    const measureCtx = document.createElement("canvas").getContext("2d")!;

    for (const rawItem of textContent.items) {
      if (!("str" in rawItem)) continue;
      const item = rawItem as { str: string; transform: number[]; width: number; height: number };
      if (!item.str) continue;

      const [a, b, , , tx, ty] = item.transform;

      // Baseline origin in viewport (CSS) pixels
      const [vx, vy] = viewport.convertToViewportPoint(tx, ty);

      // Convert a second point to get direction and font-size in viewport space
      const [vpx, vpy] = viewport.convertToViewportPoint(tx + a, ty + b);
      const fontSizePx = Math.hypot(vpx - vx, vpy - vy);
      if (fontSizePx < 1) continue;

      // Angle of text direction in CSS space
      const angleDeg = Math.atan2(vpy - vy, vpx - vx) * (180 / Math.PI);

      // Target width of the text run in viewport pixels
      const widthPx = Math.max(item.width * viewport.scale, 1);

      const span = document.createElement("span");
      span.textContent = item.str;
      span.style.position        = "absolute";
      span.style.left            = `${vx}px`;
      span.style.top             = `${vy - fontSizePx}px`;
      span.style.fontSize        = `${fontSizePx}px`;
      span.style.fontFamily      = "sans-serif";
      span.style.lineHeight      = "1";
      span.style.color           = "transparent";
      span.style.whiteSpace      = "pre";
      span.style.cursor          = "text";
      span.style.pointerEvents   = "auto";
      span.style.transformOrigin = "0% 100%";
      span.style.setProperty("user-select",         "text");
      span.style.setProperty("-webkit-user-select",  "text");

      // Scale the span horizontally so it matches the PDF text width.
      // This ensures the selection highlight covers the full visible text.
      measureCtx.font = `${fontSizePx}px sans-serif`;
      const measured = measureCtx.measureText(item.str).width;
      const scaleX = measured > 0.5 ? widthPx / measured : 1;

      const transforms: string[] = [];
      if (Math.abs(angleDeg) > 0.5) transforms.push(`rotate(${angleDeg}deg)`);
      if (Math.abs(scaleX - 1) > 0.02) transforms.push(`scaleX(${scaleX})`);
      if (transforms.length) span.style.transform = transforms.join(" ");

      fragment.appendChild(span);
    }

    this.textLayerDiv.appendChild(fragment);
  }

  /** Returns link annotations for the current page (subtype "Link" with a URL).
   *  Multi-line links may have QuadPoints; each quad becomes its own rect entry.
   *  Also detects bare URLs embedded in text content (not formal link annotations). */
  async getPageLinkAnnotations(): Promise<Array<{ url: string; rect: [number, number, number, number] }>> {
    if (!this.pdfDoc || !this._viewport) return [];
    const page = await this.pdfDoc.getPage(this._currentPage);
    const [annotations, textContent] = await Promise.all([
      page.getAnnotations(),
      page.getTextContent(),
    ]);
    const viewport = this._viewport;
    const result: Array<{ url: string; rect: [number, number, number, number] }> = [];

    // 1. Formal link annotations
    for (const ann of annotations) {
      if (ann.subtype !== "Link") continue;
      const url: string | undefined = ann.url ?? ann.unsafeUrl;
      if (!url || (!url.startsWith("http://") && !url.startsWith("https://"))) continue;
      // QuadPoints: flat array [x1,y1,x2,y2,x3,y3,x4,y4, ...] — one quad per line of text
      const quads = ann.quadPoints as number[] | undefined;
      if (quads && quads.length >= 8) {
        for (let i = 0; i + 7 < quads.length; i += 8) {
          const xs = [quads[i], quads[i + 2], quads[i + 4], quads[i + 6]];
          const ys = [quads[i + 1], quads[i + 3], quads[i + 5], quads[i + 7]];
          const pdfRect: [number, number, number, number] = [
            Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys),
          ];
          const [x1, y1, x2, y2] = viewport.convertToViewportRectangle(pdfRect);
          result.push({ url, rect: [x1, y1, x2, y2] });
        }
      } else {
        const [x1, y1, x2, y2] = viewport.convertToViewportRectangle(ann.rect as [number, number, number, number]);
        result.push({ url, rect: [x1, y1, x2, y2] });
      }
    }

    // 2. Bare URLs in text content (not wrapped in a link annotation)
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const tItems = textContent.items.filter((it): it is any => "str" in it) as
      Array<{ str: string; transform: number[]; width: number; height: number }>;
    // Build a concatenated string keeping per-item start offsets
    let combined = "";
    const offsets: Array<{ item: { str: string; transform: number[]; width: number; height: number }; start: number }> = [];
    for (const item of tItems) {
      offsets.push({ item, start: combined.length });
      combined += item.str;
    }
    const annotatedUrls = new Set(result.map((r) => r.url));
    const urlRe = /https?:\/\/[^\s)\]>"]+/g;
    let m: RegExpExecArray | null;
    while ((m = urlRe.exec(combined)) !== null) {
      const url = m[0];
      if (annotatedUrls.has(url)) continue; // already covered by a formal annotation
      const urlStart = m.index;
      const urlEnd = urlStart + url.length;
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (const { item, start } of offsets) {
        const end = start + item.str.length;
        if (end <= urlStart || start >= urlEnd) continue;
        const [, , , , x, y] = item.transform;
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x + item.width);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y + item.height);
      }
      if (!isFinite(minX)) continue;
      const [vx1, vy1, vx2, vy2] = viewport.convertToViewportRectangle([minX, minY, maxX, maxY]);
      result.push({ url, rect: [vx1, vy1, vx2, vy2] });
    }

    return result;
  }

  /** Restore a known rotation without re-rendering (call render/renderCurrentPage after). */
  restoreRotation(degrees: number): void {
    this._rotation = degrees;
  }

  /** Renders all pages as JPEG data URLs (base64 only, no prefix) for silent printing. */
  async renderAllPagesForPrint(dpi = 200): Promise<string[]> {
    if (!this.pdfDoc) return [];
    const scale = dpi / 72;
    const results: string[] = [];
    for (let i = 1; i <= this._pageCount; i++) {
      const page = await this.pdfDoc.getPage(i);
      const viewport = page.getViewport({
        scale,
        rotation: (page.rotate + this._rotation) % 360,
      });
      const canvas = document.createElement("canvas");
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      const ctx = canvas.getContext("2d")!;
      await page.render({ canvasContext: ctx, viewport }).promise;
      results.push(canvas.toDataURL("image/jpeg", 0.92).split(",")[1]);
    }
    return results;
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
    this.textLayerDiv.innerHTML = "";
  }
}
