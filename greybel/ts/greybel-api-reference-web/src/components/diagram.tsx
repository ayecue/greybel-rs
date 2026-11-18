import { useEffect, useRef, useState } from 'preact/hooks';

/** Renders Mermaid `source` as a zoomable, draggable SVG. Shows the source until rendering succeeds, or if it fails. */
export function Diagram({ source }: { source: string }) {
  const viewport = useRef<HTMLDivElement>(null);
  const view = useRef<DiagramView | undefined>(undefined);
  const [zoom, setZoom] = useState<number>();

  useEffect(() => {
    let isCurrent = true;

    void new DiagramRenderer().render(source).then((svg) => {
      if (!isCurrent || svg === undefined || viewport.current === null) return;
      view.current = new DiagramView(viewport.current, svg, setZoom);
    });

    return () => {
      isCurrent = false;
      view.current?.dispose();
      view.current = undefined;
    };
  }, [source]);

  return (
    <div class="diagram-box">
      {zoom !== undefined && (
        <div class="diagram-tools">
          <button
            type="button"
            class="action"
            aria-label="Zoom out"
            onClick={() => view.current?.zoomBy(1 / ZOOM_STEP)}
          >
            −
          </button>
          <span class="zoom">{Math.round(zoom * 100)}%</span>
          <button type="button" class="action" aria-label="Zoom in" onClick={() => view.current?.zoomBy(ZOOM_STEP)}>
            +
          </button>
          <button type="button" class="action" onClick={() => view.current?.fit()}>
            Fit
          </button>
          <button type="button" class="action" onClick={() => view.current?.zoomTo(1)}>
            100%
          </button>
        </div>
      )}
      <div class="diagram" ref={viewport} />
      {zoom === undefined && (
        <pre class="example" data-language="mermaid">
          <code>{source}</code>
        </pre>
      )}
    </div>
  );
}

const ZOOM_STEP = 1.25;
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 4;

/**
 * Shows a diagram's SVG at a zoom: buttons, Ctrl or Cmd with the mouse wheel around the pointer, and dragging to
 * move it. Starts fitted to the width, at most at 100%.
 */
class DiagramView {
  readonly #viewport: HTMLElement;
  readonly #svg: SVGSVGElement;
  readonly #width: number;
  readonly #height: number;
  readonly #onZoom: (zoom: number) => void;
  readonly #abort = new AbortController();
  #zoom = 1;
  #isZoomed = false;
  #drag: { x: number; y: number; left: number; top: number } | undefined;

  constructor(viewport: HTMLElement, svg: string, onZoom: (zoom: number) => void) {
    viewport.innerHTML = svg;
    this.#viewport = viewport;
    this.#svg = viewport.querySelector('svg')!;

    const box = this.#svg.viewBox.baseVal;

    this.#width = box.width || this.#svg.getBoundingClientRect().width;
    this.#height = box.height || this.#svg.getBoundingClientRect().height;
    this.#onZoom = onZoom;
    this.#svg.style.maxWidth = 'none';
    this.#listen();
    this.#fit();
    this.#observer.observe(viewport);
    onZoom(this.#zoom);
  }

  /** Refits on resize until zoom is explicitly changed, such as when a hidden tab appears. */
  readonly #observer = new ResizeObserver(() => {
    if (this.#isZoomed) return;
    this.#fit();
  });

  dispose(): void {
    this.#abort.abort();
    this.#observer.disconnect();
  }

  fit(): void {
    this.#isZoomed = true;
    this.#fit();
  }

  zoomBy(factor: number): void {
    this.zoomTo(this.#zoom * factor);
  }

  /** The point at `anchorX`, `anchorY` of the viewport stays in place; by default the viewport's center. */
  zoomTo(zoom: number, anchorX = this.#viewport.clientWidth / 2, anchorY = this.#viewport.clientHeight / 2): void {
    this.#isZoomed = true;
    this.#setZoom(zoom, anchorX, anchorY);
  }

  #fit(): void {
    if (this.#viewport.clientWidth === 0) return;
    this.#setZoom(Math.min(1, this.#viewport.clientWidth / this.#width), 0, 0);
  }

  #setZoom(zoom: number, anchorX: number, anchorY: number): void {
    const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
    const scale = next / this.#zoom;
    const x = (this.#viewport.scrollLeft + anchorX) * scale - anchorX;
    const y = (this.#viewport.scrollTop + anchorY) * scale - anchorY;

    this.#zoom = next;
    this.#svg.style.width = `${this.#width * next}px`;
    this.#svg.style.height = `${this.#height * next}px`;
    this.#viewport.scrollLeft = x;
    this.#viewport.scrollTop = y;
    this.#onZoom(next);
  }

  #listen(): void {
    const options = { signal: this.#abort.signal };
    const viewport = this.#viewport;

    viewport.addEventListener(
      'wheel',
      (event) => {
        if (!event.ctrlKey && !event.metaKey) return;
        event.preventDefault();

        const bounds = viewport.getBoundingClientRect();

        this.zoomTo(
          this.#zoom * Math.exp(-event.deltaY / 300),
          event.clientX - bounds.left,
          event.clientY - bounds.top
        );
      },
      { ...options, passive: false }
    );
    viewport.addEventListener(
      'pointerdown',
      (event) => {
        if (event.button !== 0) return;
        this.#drag = { x: event.clientX, y: event.clientY, left: viewport.scrollLeft, top: viewport.scrollTop };
        viewport.setPointerCapture(event.pointerId);
        viewport.classList.add('dragging');
      },
      options
    );
    viewport.addEventListener(
      'pointermove',
      (event) => {
        if (this.#drag === undefined) return;
        viewport.scrollLeft = this.#drag.left - (event.clientX - this.#drag.x);
        viewport.scrollTop = this.#drag.top - (event.clientY - this.#drag.y);
      },
      options
    );
    for (const type of ['pointerup', 'pointercancel'] as const) {
      viewport.addEventListener(
        type,
        () => {
          this.#drag = undefined;
          viewport.classList.remove('dragging');
        },
        options
      );
    }
  }
}

type Mermaid = typeof import('mermaid').default;

/** Mermaid ES build matching the `mermaid` dev dependency; loaded on pages with diagrams. */
export const MERMAID_URL = 'https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.esm.min.mjs';

/** Loads Mermaid once from `MERMAID_URL` and renders diagrams with ids unique on the page. */
class DiagramRenderer {
  static #mermaid: Promise<Mermaid> | undefined;
  static #count = 0;

  async render(source: string): Promise<string | undefined> {
    try {
      const mermaid = await DiagramRenderer.#load();
      const { svg } = await mermaid.render(`reference-diagram-${DiagramRenderer.#count++}`, source);

      return svg;
    } catch {
      return undefined;
    }
  }

  static #load(): Promise<Mermaid> {
    DiagramRenderer.#mermaid ??= (import(/* @vite-ignore */ MERMAID_URL) as Promise<{ default: Mermaid }>).then(
      ({ default: mermaid }) => {
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: 'dark',
          themeVariables: { background: '#040719' },
          flowchart: { useMaxWidth: false }
        });

        return mermaid;
      }
    );

    return DiagramRenderer.#mermaid;
  }
}
