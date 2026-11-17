import type { RefObject } from 'preact';
import { useEffect, useLayoutEffect } from 'preact/hooks';

/** Gap between a fitted element and the bottom of the visible area, in pixels. */
const MARGIN = 8;

/** The visible area of an element's nearest scrolling ancestor, or of its window. */
class ViewMeasure {
  readonly #element: HTMLElement;
  readonly #container: HTMLElement | Window;

  constructor(element: HTMLElement) {
    this.#element = element;
    this.#container = this.#findScrollContainer();
  }

  get top(): number {
    return 'getBoundingClientRect' in this.#container ? this.#container.getBoundingClientRect().top : 0;
  }

  get bottom(): number {
    return 'getBoundingClientRect' in this.#container
      ? this.#container.getBoundingClientRect().bottom
      : this.#container.innerHeight;
  }

  /** Calls `update` now and whenever the container scrolls or it or the window resizes; returns the cleanup. */
  observe(update: () => void): () => void {
    const view = this.#element.ownerDocument.defaultView ?? window;
    const observer = new ResizeObserver(update);

    update();
    this.#container.addEventListener('scroll', update, { passive: true });
    view.addEventListener('resize', update);
    if (this.#container !== view) observer.observe(this.#container as HTMLElement);

    return () => {
      this.#container.removeEventListener('scroll', update);
      view.removeEventListener('resize', update);
      observer.disconnect();
    };
  }

  #findScrollContainer(): HTMLElement | Window {
    const view = this.#element.ownerDocument.defaultView ?? window;

    for (let parent = this.#element.parentElement; parent !== null; parent = parent.parentElement) {
      const { overflowY } = view.getComputedStyle(parent);

      if (overflowY === 'auto' || overflowY === 'scroll') return parent;
    }

    return view;
  }
}

/**
 * Sets `--reference-view-height` on `root` to the height of its scroll container: the nearest scrolling ancestor, such
 * as an editor panel, or the window.
 */
export function useViewHeight(root: RefObject<HTMLElement | null>, isReady: boolean): void {
  useEffect(() => {
    const element = root.current;

    if (element === null) return;

    const measure = new ViewMeasure(element);
    const update = () => element.style.setProperty('--reference-view-height', `${measure.bottom - measure.top}px`);

    return measure.observe(update);
  }, [isReady]);
}

/** Limits the height of `target` while `isOpen`, so its bottom stays inside the visible area of its scroll container. */
export function useFitToView(target: RefObject<HTMLElement | null>, isOpen: boolean): void {
  useLayoutEffect(() => {
    const element = target.current;

    if (!isOpen || element === null) return;

    const measure = new ViewMeasure(element);
    const update = () => {
      const height = measure.bottom - element.getBoundingClientRect().top - MARGIN;

      element.style.maxHeight = `${Math.max(height, 0)}px`;
    };

    return measure.observe(update);
  }, [isOpen]);
}
