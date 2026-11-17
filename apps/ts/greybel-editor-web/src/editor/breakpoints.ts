import { RangeSet, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, GutterMarker, gutter, type DecorationSet } from '@codemirror/view';

class BreakpointMarker extends GutterMarker {
  override toDOM(): Node {
    const marker = document.createElement('span');

    marker.className = 'cm-breakpoint';

    return marker;
  }
}

const BREAKPOINT = new BreakpointMarker();
const STOP_LINE = Decoration.line({ class: 'cm-stop-line' });

/** The breakpoint gutter and the stopped-line marker. All lines are one-based. */
export class BreakpointGutter {
  readonly #toggle = StateEffect.define<number>();
  readonly #setStopLine = StateEffect.define<number | undefined>();
  /** Keyed by line start position. */
  readonly #field = StateField.define<RangeSet<GutterMarker>>({
    create: () => RangeSet.empty,
    update: (breakpoints, transaction) => {
      let next = breakpoints.map(transaction.changes);

      for (const effect of transaction.effects) {
        if (!effect.is(this.#toggle)) continue;

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

  readonly #stopLineField = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update: (stopLine, transaction) => {
      let next = stopLine.map(transaction.changes);

      for (const effect of transaction.effects) {
        if (!effect.is(this.#setStopLine)) continue;
        next =
          effect.value === undefined
            ? Decoration.none
            : Decoration.set([STOP_LINE.range(transaction.state.doc.line(effect.value).from)]);
      }

      return next;
    },
    provide: (field) => EditorView.decorations.from(field)
  });

  extension(onChange: (lines: number[]) => void): Extension {
    return [
      this.#field,
      this.#stopLineField,
      gutter({
        class: 'cm-breakpoint-gutter',
        markers: (view) => view.state.field(this.#field),
        initialSpacer: () => BREAKPOINT,
        domEventHandlers: {
          mousedown: (view, line) => {
            view.dispatch({ effects: this.#toggle.of(line.from) });
            onChange(this.getLines(view.state));

            return true;
          }
        }
      }),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged || update.state.field(this.#field).size === 0) return;
        onChange(this.getLines(update.state));
      })
    ];
  }

  getLines(state: EditorState): number[] {
    const lines: number[] = [];
    const cursor = state.field(this.#field).iter();

    for (; cursor.value !== null; cursor.next()) lines.push(state.doc.lineAt(cursor.from).number);

    return lines;
  }

  /** Adds breakpoints at `lines`. */
  restore(state: EditorState, lines: readonly number[]): EditorState {
    const effects = lines
      .filter((line) => line >= 1 && line <= state.doc.lines)
      .map((line) => this.#toggle.of(state.doc.line(line).from));

    return effects.length === 0 ? state : state.update({ effects }).state;
  }

  /** `undefined` clears the marker. */
  showStopLine(view: EditorView, line: number | undefined): void {
    const isInDocument = line !== undefined && line >= 1 && line <= view.state.doc.lines;
    const effects = [this.#setStopLine.of(isInDocument ? line : undefined)];

    view.dispatch({
      effects: isInDocument
        ? [...effects, EditorView.scrollIntoView(view.state.doc.line(line).from, { y: 'center' })]
        : effects
    });
  }
}
