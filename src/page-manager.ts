import type { PDFDocumentProxy } from "pdfjs-dist";
import type { Translations } from "./i18n";
import type { PageOperation } from "./tauri-bridge";
import { ICON_DELETE, ICON_ROTATE_CW } from "./icons";

interface PageState {
  page: number;
  rotation: number;
  deleted: boolean;
}

type ConfirmHandler = (operations: PageOperation[]) => void;

export class PageManagerModal {
  private backdrop: HTMLElement;
  private grid: HTMLElement;
  private titleEl: HTMLElement;
  private cancelBtn: HTMLButtonElement;
  private applyBtn: HTMLButtonElement;
  private pageStates: PageState[] = [];
  private cards: HTMLElement[] = [];
  private confirmHandlers: ConfirmHandler[] = [];
  private _i18nText = new Map<HTMLElement, keyof Translations>();
  private _i18nTitle = new Map<HTMLElement, keyof Translations>();

  constructor() {
    const { backdrop, grid, titleEl, cancelBtn, applyBtn } = this.buildDOM();
    this.backdrop = backdrop;
    this.grid = grid;
    this.titleEl = titleEl;
    this.cancelBtn = cancelBtn;
    this.applyBtn = applyBtn;
    document.body.appendChild(this.backdrop);
  }

  onConfirm(cb: ConfirmHandler): void {
    this.confirmHandlers.push(cb);
  }

  applyTranslations(t: Translations): void {
    this._i18nText.forEach((key, el) => {
      el.textContent = t[key] ?? null;
    });
    this._i18nTitle.forEach((key, el) => {
      el.title = t[key] ?? "";
    });
  }

  async open(pdfDoc: PDFDocumentProxy, pageCount: number): Promise<void> {
    this.pageStates = [];
    this.cards = [];
    this.grid.innerHTML = "";

    for (let i = 1; i <= pageCount; i++) {
      this.pageStates.push({ page: i, rotation: 0, deleted: false });
    }

    this.backdrop.classList.remove("hidden");

    // Render thumbnails
    for (let i = 0; i < pageCount; i++) {
      const state = this.pageStates[i];
      const card = this.createCard(state, pdfDoc);
      this.cards.push(card);
      this.grid.appendChild(card);
    }
  }

  close(): void {
    this.backdrop.classList.add("hidden");
    this.grid.innerHTML = "";
    this.cards = [];
    this.pageStates = [];
  }

  private buildDOM() {
    const backdrop = document.createElement("div");
    backdrop.className = "modal-backdrop hidden";

    const container = document.createElement("div");
    container.className = "page-manager-container";

    // Header
    const header = document.createElement("div");
    header.className = "page-manager-header";

    const titleEl = document.createElement("span");
    titleEl.textContent = "Manage Pages";
    this._i18nText.set(titleEl, "pmTitle");

    const closeBtn = document.createElement("button");
    closeBtn.className = "icon-btn";
    closeBtn.innerHTML = "&times;";
    closeBtn.title = "Close";
    closeBtn.addEventListener("click", () => this.close());

    header.append(titleEl, closeBtn);

    // Grid
    const grid = document.createElement("div");
    grid.className = "page-manager-grid";

    // Footer
    const footer = document.createElement("div");
    footer.className = "page-manager-footer";

    const cancelBtn = document.createElement("button");
    cancelBtn.className = "btn";
    cancelBtn.textContent = "Cancel";
    this._i18nText.set(cancelBtn, "pmCancel");
    cancelBtn.addEventListener("click", () => this.close());

    const applyBtn = document.createElement("button");
    applyBtn.className = "btn btn-primary";
    applyBtn.textContent = "Apply";
    this._i18nText.set(applyBtn, "pmApply");
    applyBtn.addEventListener("click", () => this.handleApply());

    footer.append(cancelBtn, applyBtn);
    container.append(header, grid, footer);

    backdrop.addEventListener("click", (e) => {
      if (e.target === backdrop) this.close();
    });

    backdrop.appendChild(container);

    return { backdrop, grid, titleEl, cancelBtn, applyBtn };
  }

  private createCard(state: PageState, pdfDoc: PDFDocumentProxy): HTMLElement {
    const card = document.createElement("div");
    card.className = "page-thumbnail-card";

    const canvasWrapper = document.createElement("div");
    canvasWrapper.className = "page-thumbnail-canvas-wrapper";

    const canvas = document.createElement("canvas");
    canvasWrapper.appendChild(canvas);

    // Render thumbnail async
    void this.renderThumbnail(state.page, canvas, pdfDoc);

    // Controls
    const controls = document.createElement("div");
    controls.className = "page-thumbnail-controls";

    const pageNum = document.createElement("span");
    pageNum.className = "page-number";
    pageNum.textContent = String(state.page);

    const rotateBtn = document.createElement("button");
    rotateBtn.className = "icon-btn";
    rotateBtn.innerHTML = ICON_ROTATE_CW;
    rotateBtn.title = "Rotate page";
    this._i18nTitle.set(rotateBtn, "pmRotatePage");
    rotateBtn.addEventListener("click", () => {
      state.rotation = (state.rotation + 90) % 360;
      canvasWrapper.style.transform = state.rotation ? `rotate(${state.rotation}deg)` : "";
    });

    const deleteBtn = document.createElement("button");
    deleteBtn.className = "icon-btn";
    deleteBtn.innerHTML = ICON_DELETE;
    deleteBtn.title = "Delete page";
    this._i18nTitle.set(deleteBtn, "pmDeletePage");
    deleteBtn.addEventListener("click", () => {
      state.deleted = !state.deleted;
      card.classList.toggle("deleted", state.deleted);
      this.updateDeleteButtons();
    });

    controls.append(pageNum, rotateBtn, deleteBtn);
    card.append(canvasWrapper, controls);

    return card;
  }

  private async renderThumbnail(
    pageNum: number,
    canvas: HTMLCanvasElement,
    pdfDoc: PDFDocumentProxy,
  ): Promise<void> {
    const page = await pdfDoc.getPage(pageNum);
    const desiredWidth = 150;
    const baseViewport = page.getViewport({ scale: 1.0 });
    const scale = desiredWidth / baseViewport.width;
    const viewport = page.getViewport({ scale });

    canvas.width = viewport.width;
    canvas.height = viewport.height;
    canvas.style.width = `${viewport.width}px`;
    canvas.style.height = `${viewport.height}px`;

    const ctx = canvas.getContext("2d")!;
    await page.render({ canvasContext: ctx, viewport }).promise;
  }

  private updateDeleteButtons(): void {
    const aliveCount = this.pageStates.filter((s) => !s.deleted).length;
    // Prevent deleting all pages: disable delete buttons when only 1 alive
    for (let i = 0; i < this.cards.length; i++) {
      const state = this.pageStates[i];
      const deleteBtn = this.cards[i].querySelector(
        ".page-thumbnail-controls .icon-btn:last-child",
      ) as HTMLButtonElement | null;
      if (deleteBtn) {
        deleteBtn.disabled = !state.deleted && aliveCount <= 1;
      }
    }
  }

  private handleApply(): void {
    const operations: PageOperation[] = this.pageStates.map((s) => ({
      page: s.page,
      rotation: s.rotation,
      delete: s.deleted,
    }));
    for (const cb of this.confirmHandlers) cb(operations);
    this.close();
  }
}
