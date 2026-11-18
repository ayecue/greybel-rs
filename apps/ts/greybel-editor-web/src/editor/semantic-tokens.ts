import { LSPPlugin } from '@codemirror/lsp-client';
import { RangeSetBuilder, StateEffect, StateField, type Extension, type Text } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from '@codemirror/view';

const DELAY = 150;

interface SemanticTokenResponse {
  readonly data: readonly number[];
}

interface SemanticTokensLegend {
  readonly tokenTypes: readonly string[];
  readonly tokenModifiers?: readonly string[];
}

/** Bracket colors cycle through this many classes by nesting depth, as in Grey Hack's code editor. */
const BRACKET_DEPTHS = 6;

const BRACKET_KINDS = [
  { name: 'paren', open: '(', close: ')' },
  { name: 'square', open: '[', close: ']' }
] as const;

/**
 * Marks the language server's semantic tokens with one class per token type: `cm-token-<type>`, plus
 * `cm-token-default-library` for API function names, `cm-<paren|square>-<1..6>` by bracket depth and
 * `cm-bracket-unmatched` for a closing bracket without an opening one.
 */
export class SemanticTokenHighlighter {
  readonly #setTokens = StateEffect.define<DecorationSet>();
  readonly #field = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update: (tokens, transaction) => {
      let next = tokens.map(transaction.changes);

      for (const effect of transaction.effects) {
        if (!effect.is(this.#setTokens)) continue;
        next = effect.value;
      }

      return next;
    },
    provide: (field) => EditorView.decorations.from(field)
  });

  extension(): Extension {
    const highlighter = this;

    return [
      this.#field,
      ViewPlugin.fromClass(
        class {
          #timer: ReturnType<typeof setTimeout> | undefined;
          #isDestroyed = false;

          constructor(view: EditorView) {
            this.#schedule(view);
          }

          update(update: ViewUpdate): void {
            if (!update.docChanged) return;
            this.#schedule(update.view);
          }

          destroy(): void {
            this.#isDestroyed = true;
            clearTimeout(this.#timer);
          }

          #schedule(view: EditorView): void {
            clearTimeout(this.#timer);
            this.#timer = setTimeout(() => void this.#request(view), DELAY);
          }

          async #request(view: EditorView): Promise<void> {
            const plugin = LSPPlugin.get(view);

            if (plugin === null) return;
            await plugin.client.initializing;

            const provider = plugin.client.serverCapabilities?.semanticTokensProvider as
              { readonly legend: SemanticTokensLegend } | undefined;

            if (provider === undefined || this.#isDestroyed) return;
            plugin.client.sync();

            const doc = view.state.doc;
            const tokens = await plugin.client
              .request<unknown, SemanticTokenResponse | null>('textDocument/semanticTokens/full', {
                textDocument: { uri: plugin.uri }
              })
              .catch(() => null);

            if (tokens === null || this.#isDestroyed || view.state.doc !== doc) return;
            view.dispatch({
              effects: highlighter.#setTokens.of(highlighter.#createMarks(tokens.data, provider.legend, plugin, doc))
            });
          }
        }
      )
    ];
  }

  #createMarks(data: readonly number[], legend: SemanticTokensLegend, plugin: LSPPlugin, doc: Text): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    const libraryBit = 1 << (legend.tokenModifiers ?? []).indexOf('defaultLibrary');
    const depths = new Map<string, number>(BRACKET_KINDS.map((kind) => [kind.name, 0]));
    let line = 0;
    let character = 0;

    for (let index = 0; index + 4 < data.length; index += 5) {
      const deltaLine = data[index]!;
      const deltaStart = data[index + 1]!;

      line += deltaLine;
      character = deltaLine === 0 ? character + deltaStart : deltaStart;

      const from = plugin.fromPosition({ line, character }, doc);
      const to = plugin.fromPosition({ line, character: character + data[index + 2]! }, doc);
      const type = legend.tokenTypes[data[index + 3]!];

      if (type === undefined || to <= from) continue;

      const classes = [`cm-token-${type}`];

      if (libraryBit > 0 && (data[index + 4]! & libraryBit) !== 0) classes.push('cm-token-default-library');
      if (type === 'punctuator') classes.push(...this.#getBracketClasses(doc.sliceString(from, to), depths));
      builder.add(from, to, Decoration.mark({ class: classes.join(' ') }));
    }

    return builder.finish();
  }

  /** Returns the classes for bracket `text` and updates `depths`; nesting colors match Grey Hack's editor. */
  #getBracketClasses(text: string, depths: Map<string, number>): string[] {
    const kind = BRACKET_KINDS.find((candidate) => text === candidate.open || text === candidate.close);

    if (kind === undefined) return text === ':' ? ['cm-token-colon'] : [];

    const depth = depths.get(kind.name)! + (text === kind.open ? 1 : 0);

    depths.set(kind.name, text === kind.close ? depth - 1 : depth);

    return depth < 1 ? ['cm-bracket-unmatched'] : [`cm-${kind.name}-${((depth - 1) % BRACKET_DEPTHS) + 1}`];
  }
}
