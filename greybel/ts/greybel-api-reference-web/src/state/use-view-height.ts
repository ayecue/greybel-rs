import type { RefObject } from 'preact';
import { useEffect, useLayoutEffect } from 'preact/hooks';

/** Gap between a fitted element and the bottom of the visible area, in pixels. */
const MARGIN = 8;

/**
 * Sets `--reference-view-height` on `root` to the height of its scroll container: the nearest scrolling ancestor, such
 * as an editor panel, or the window.
 */
export function useViewHeight(root: RefObject<HTMLElement | null>, isRendered: boolean): void {
  useEffect(() => {
    const element = root.current;

    if (element === null) return;

    const container = findScrollContainer(element);
    const update = () =>
      element.style.setProperty('--reference-view-height', `${getViewBottom(container) - getViewTop(container)}px`);

    return observeView(container, element, update);
  }, [isRendered]);
}

/** Limits the height of `target` while `isOpen`, so its bottom stays inside the visible area of its scroll container. */
export function useFitToView(target: RefObject<HTMLElement | null>, isOpen: boolean): void {
  useLayoutEffect(() => {
    const element = target.current;

    if (!isOpen || element === null) return;

    const container = findScrollContainer(element);
    const update = () => {
      const height = getViewBottom(container) - element.getBoundingClientRect().top - MARGIN;

      element.style.maxHeight = `${Math.max(height, 0)}px`;
    };

    return observeView(container, element, update);
  }, [isOpen]);
}

/** The nearest ancestor of `element` that scrolls vertically; the window when none does. */
function findScrollContainer(element: HTMLElement): HTMLElement | Window {
  const view = element.ownerDocument.defaultView ?? window;

  for (let parent = element.parentElement; parent !== null; parent = parent.parentElement) {
    const { overflowY } = view.getComputedStyle(parent);

    if (overflowY === 'auto' || overflowY === 'scroll') return parent;
  }

  return view;
}

function getViewTop(container: HTMLElement | Window): number {
  return 'getBoundingClientRect' in container ? container.getBoundingClientRect().top : 0;
}

function getViewBottom(container: HTMLElement | Window): number {
  return 'getBoundingClientRect' in container ? container.getBoundingClientRect().bottom : container.innerHeight;
}

/** Calls `update` now and whenever `container` scrolls or it or the window resizes; returns the cleanup. */
function observeView(container: HTMLElement | Window, element: HTMLElement, update: () => void): () => void {
  const view = element.ownerDocument.defaultView ?? window;
  const observer = new ResizeObserver(update);

  update();
  container.addEventListener('scroll', update, { passive: true });
  view.addEventListener('resize', update);
  if (container !== view) observer.observe(container as HTMLElement);

  return () => {
    container.removeEventListener('scroll', update);
    view.removeEventListener('resize', update);
    observer.disconnect();
  };
}
