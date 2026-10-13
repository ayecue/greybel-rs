import { Block, type Span } from '../protocol.generated.js';

/** Lowercase searchable text from `blocks`, including spans, list items, table cells, and code. */
export function blockText(blocks: readonly Block[]): string {
  return blocks.map(textOf).join('\n').toLowerCase();
}

function textOf(block: Block): string {
  if (block instanceof Block.Paragraph || block instanceof Block.Heading) return spansText(block.spans);
  if (block instanceof Block.List) return block.items.map((item) => spansText(item.spans)).join('\n');
  if (block instanceof Block.Code) return block.tokens.map((token) => token.text).join('');
  if (block instanceof Block.Table) {
    return [block.head, ...block.rows.map((row) => row.cells)]
      .map((cells) => cells.map((cell) => spansText(cell.spans)).join(' '))
      .join('\n');
  }

  return '';
}

function spansText(spans: readonly Span[]): string {
  return spans.map((span) => span.text).join('');
}
