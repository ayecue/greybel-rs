import { useRef } from 'preact/hooks';

import { useServices } from './services.js';

const STEP = 16;

export interface SplitterConfiguration {
  /** Sets the CSS variable `--<name>-size`. */
  readonly name: string;
  readonly axis: 'x' | 'y';
  /** The neighbour the size applies to. */
  readonly side: 'before' | 'after';
  /** Minimum size of each neighbour in pixels. */
  readonly min: number;
}

export function Splitter({ name, axis, side, min }: SplitterConfiguration) {
  const ref = useRef<HTMLDivElement>(null);
  const { layout } = useServices();

  const getNeighbours = () => {
    const element = ref.current;
    const before = element?.previousElementSibling;
    const after = element?.nextElementSibling;

    if (!(before instanceof HTMLElement) || !(after instanceof HTMLElement)) return undefined;

    return side === 'before' ? { target: before, other: after } : { target: after, other: before };
  };

  const measure = (element: HTMLElement) => {
    const box = element.getBoundingClientRect();

    return axis === 'x' ? box.width : box.height;
  };

  const resize = (size: number, total: number) => {
    const limit = Math.round(Math.min(Math.max(size, min), Math.max(min, total - min)));

    layout.show(name, limit);

    return limit;
  };

  const onPointerDown = (event: PointerEvent) => {
    const parts = getNeighbours();

    if (parts === undefined || event.button !== 0) return;
    event.preventDefault();

    const element = event.currentTarget as HTMLElement;
    const start = axis === 'x' ? event.clientX : event.clientY;
    const startSize = measure(parts.target);
    const total = startSize + measure(parts.other);
    const direction = side === 'before' ? 1 : -1;
    let size = startSize;

    const onMove = (move: PointerEvent) => {
      const position = axis === 'x' ? move.clientX : move.clientY;

      size = resize(startSize + (position - start) * direction, total);
    };

    const onUp = () => {
      element.removeEventListener('pointermove', onMove);
      element.removeEventListener('pointerup', onUp);
      element.removeEventListener('pointercancel', onUp);
      element.classList.remove('dragging');
      layout.store(name, size);
    };

    element.setPointerCapture(event.pointerId);
    element.classList.add('dragging');
    element.addEventListener('pointermove', onMove);
    element.addEventListener('pointerup', onUp);
    element.addEventListener('pointercancel', onUp);
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const keys = axis === 'x' ? ['ArrowLeft', 'ArrowRight'] : ['ArrowUp', 'ArrowDown'];
    const index = keys.indexOf(event.key);
    const parts = getNeighbours();

    if (index < 0 || parts === undefined) return;
    event.preventDefault();

    const direction = (index === 0 ? -1 : 1) * (side === 'before' ? 1 : -1);
    const size = measure(parts.target);

    layout.store(name, resize(size + STEP * direction, size + measure(parts.other)));
  };

  return (
    <div
      ref={ref}
      class={`splitter splitter-${axis}`}
      data-name={name}
      role="separator"
      aria-orientation={axis === 'x' ? 'vertical' : 'horizontal'}
      tabIndex={0}
      title="Drag to resize; double-click to reset"
      onPointerDown={onPointerDown}
      onKeyDown={onKeyDown}
      onDblClick={() => layout.reset(name)}
    />
  );
}
