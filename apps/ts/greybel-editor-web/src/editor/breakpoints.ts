import {
  RangeSet,
  StateEffect,
  StateField,
  type EditorState,
  type Extension,
  type StateEffectType
} from '@codemirror/state';
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
const FRAME_LINE = Decoration.line({ class: 'cm-frame-line' });

/** The breakpoint gutter, the stopped-line marker and the selected caller frame's marker. All lines are one-based. */
export class BreakpointGutter {
  readonly #toggle = StateEffect.define<number>();
  readonly #setStopLine = StateEffect.define<number | undefined>();
  readonly #setFrameLine = StateEffect.define<number | undefined>();
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

  readonly #stopLineField = this.#defineLineField(this.#setStopLine, STOP_LINE);
  readonly #frameLineField = this.#defineLineField(this.#setFrameLine, FRAME_LINE);

  extension(onChange: (lines: number[]) => void): Extension {
    return [
      this.#field,
      this.#stopLineField,
      this.#frameLineField,
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

  /** Marks the stopped line and the selected caller frame's line, and scrolls to the frame's or else the stop's. */
  showStopLines(view: EditorView, stopLine: number | undefined, frameLine: number | undefined): void {
    const isInDocument = (line: number | undefined): line is number =>
      line !== undefined && line >= 1 && line <= view.state.doc.lines;
    const shownStop = isInDocument(stopLine) ? stopLine : undefined;
    const shownFrame = isInDocument(frameLine) ? frameLine : undefined;
    const target = shownFrame ?? shownStop;
    const effects = [this.#setStopLine.of(shownStop), this.#setFrameLine.of(shownFrame)];

    view.dispatch({
      effects:
        target === undefined
          ? effects
          : [...effects, EditorView.scrollIntoView(view.state.doc.line(target).from, { y: 'center' })]
    });
  }

  #defineLineField(setLine: StateEffectType<number | undefined>, decoration: Decoration): StateField<DecorationSet> {
    return StateField.define<DecorationSet>({
      create: () => Decoration.none,
      update: (lines, transaction) => {
        let next = lines.map(transaction.changes);

        for (const effect of transaction.effects) {
          if (!effect.is(setLine)) continue;
          next =
            effect.value === undefined
              ? Decoration.none
              : Decoration.set([decoration.range(transaction.state.doc.line(effect.value).from)]);
        }

        return next;
      },
      provide: (field) => EditorView.decorations.from(field)
    });
  }
}
