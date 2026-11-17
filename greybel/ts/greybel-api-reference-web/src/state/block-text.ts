import { Block, type Span } from '../protocol.generated.js';

/** Lowercase searchable text from blocks, including spans, list items, table cells, and code. */
export class BlockText {
  readonly #blocks: readonly Block[];

  constructor(blocks: readonly Block[]) {
    this.#blocks = blocks;
  }

  get text(): string {
    return this.#blocks
      .map((block) => this.#getText(block))
      .join('\n')
      .toLowerCase();
  }

  #getText(block: Block): string {
    if (block instanceof Block.Paragraph || block instanceof Block.Heading) return this.#getSpansText(block.spans);
    if (block instanceof Block.List) return block.items.map((item) => this.#getSpansText(item.spans)).join('\n');
    if (block instanceof Block.Code) return block.tokens.map((token) => token.text).join('');
    if (block instanceof Block.Table) {
      return [block.head, ...block.rows.map((row) => row.cells)]
        .map((cells) => cells.map((cell) => this.#getSpansText(cell.spans)).join(' '))
        .join('\n');
    }

    return '';
  }

  #getSpansText(spans: readonly Span[]): string {
    return spans.map((span) => span.text).join('');
  }
}
