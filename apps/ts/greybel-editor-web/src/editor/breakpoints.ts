import { RangeSet, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, GutterMarker, gutter, type DecorationSet } from '@codemirror/view';

const toggleBreakpoint = StateEffect.define<number>();
const setStopLine = StateEffect.define<number | undefined>();

class BreakpointMarker extends GutterMarker {
  override toDOM(): Node {
    const marker = document.createElement('span');

    marker.className = 'cm-breakpoint';

    return marker;
  }
}

const BREAKPOINT = new BreakpointMarker();

/** Breakpoint lines keyed by line-start position. */
const breakpointField = StateField.define<RangeSet<GutterMarker>>({
  create: () => RangeSet.empty,
  update(breakpoints, transaction) {
    let next = breakpoints.map(transaction.changes);

    for (const effect of transaction.effects) {
      if (!effect.is(toggleBreakpoint)) continue;

      let isSet = false;

      next.between(effect.value, effect.value, () => {
        isSet = true;
      });
      next = isSet
        ? next.update({ filter: (from) => from !== effect.value })
        : next.update({ add: [BREAKPOINT.range(effect.value)] });
    }

    return next;
  }
});

const STOP_LINE = Decoration.line({ class: 'cm-stop-line' });

/** Stopped program line, valid while the program is stopped. */
const stopLineField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(stopLine, transaction) {
    let next = stopLine.map(transaction.changes);

    for (const effect of transaction.effects) {
      if (!effect.is(setStopLine)) continue;
      next =
        effect.value === undefined
          ? Decoration.none
          : Decoration.set([STOP_LINE.range(transaction.state.doc.line(effect.value).from)]);
    }

    return next;
  },
  provide: (field) => EditorView.decorations.from(field)
});

/** One-based breakpoint lines in `state`. */
export function breakpointLines(state: EditorState): number[] {
  const lines: number[] = [];
  const cursor = state.field(breakpointField).iter();

  for (; cursor.value !== null; cursor.next()) lines.push(state.doc.lineAt(cursor.from).number);

  return lines;
}

/**
 * Returns `state` with additional one-based breakpoints at `lines`, such as breakpoints from the replaced
 * file.
 */
export function withBreakpoints(state: EditorState, lines: readonly number[]): EditorState {
  const effects = lines
    .filter((line) => line >= 1 && line <= state.doc.lines)
    .map((line) => toggleBreakpoint.of(state.doc.line(line).from));

  return effects.length === 0 ? state : state.update({ effects }).state;
}

/** Marks one-based `line` as stopped, or clears the stopped marker when `line` is undefined. */
export function showStopLine(view: EditorView, line: number | undefined): void {
  const isInDocument = line !== undefined && line >= 1 && line <= view.state.doc.lines;
  const effects = [setStopLine.of(isInDocument ? line : undefined)];

  view.dispatch({
    effects: isInDocument
      ? [...effects, EditorView.scrollIntoView(view.state.doc.line(line).from, { y: 'center' })]
      : effects
  });
}

/**
 * Breakpoint gutter that toggles breakpoints on click, reports changes through `onChange`, and marks the
 * stopped line.
 */
export function breakpoints(onChange: (lines: number[]) => void): Extension {
  return [
    breakpointField,
    stopLineField,
    gutter({
      class: 'cm-breakpoint-gutter',
      markers: (view) => view.state.field(breakpointField),
      initialSpacer: () => BREAKPOINT,
      domEventHandlers: {
        mousedown(view, line) {
          view.dispatch({ effects: toggleBreakpoint.of(line.from) });
          onChange(breakpointLines(view.state));

          return true;
        }
      }
    }),
    EditorView.updateListener.of((update) => {
      if (!update.docChanged || update.state.field(breakpointField).size === 0) return;
      onChange(breakpointLines(update.state));
    })
  ];
}
