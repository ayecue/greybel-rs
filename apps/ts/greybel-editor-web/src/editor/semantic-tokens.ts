import { LSPPlugin } from '@codemirror/lsp-client';
import { RangeSetBuilder, StateEffect, StateField, type Extension, type Text } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from '@codemirror/view';

const DELAY = 150;

interface SemanticTokenResponse {
  readonly data: readonly number[];
}

interface SemanticTokensLegend {
  readonly tokenTypes: readonly string[];
}

/** Marks the language server's semantic tokens with one class per token type: `cm-token-<type>`. */
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
      builder.add(from, to, Decoration.mark({ class: `cm-token-${type}` }));
    }

    return builder.finish();
  }
}
