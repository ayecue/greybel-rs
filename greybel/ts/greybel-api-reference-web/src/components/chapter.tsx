import { memo } from 'preact/compat';

import { Block } from '../protocol.generated.js';
import { BlockText } from '../state/block-text.js';
import type { ChapterContent } from '../state/use-guide.js';
import { Description } from './description.js';

/** A part of a chapter: one of its level-2 headings and the blocks up to the next. */
export interface ChapterPart {
  readonly anchor: string;
  readonly title: string;
  readonly blocks: readonly Block[];
  /** The part's text, in lowercase, as the guide's search reads it. */
  readonly text: string;
}

/** A chapter split at its level-2 headings into a lead and parts, with heading anchors and searchable text. */
export class ChapterStructure {
  readonly anchor: string;
  readonly title: string;
  readonly lead: readonly Block[];
  /** The title and the lead's text, in lowercase, as the guide's search reads them. */
  readonly leadText: string;
  readonly parts: readonly ChapterPart[];
  readonly headingIds: ReadonlyMap<Block, string>;

  constructor(chapter: ChapterContent) {
    const headingIds = new Map<Block, string>();
    const lead: Block[] = [];
    const parts: { anchor: string; title: string; blocks: Block[] }[] = [];
    let title: string | undefined;

    for (const block of chapter.blocks) {
      if (block instanceof Block.Heading && block.level === 1 && title === undefined) {
        title = this.#getText(block);
        headingIds.set(block, chapter.anchor);
      } else if (block instanceof Block.Heading && block.level === 2) {
        const text = this.#getText(block);
        const anchor = this.#getUniqueAnchor(`${chapter.anchor}-${this.#toSlug(text)}`, parts);

        headingIds.set(block, anchor);
        parts.push({ anchor, title: text, blocks: [] });
      }

      (parts.at(-1)?.blocks ?? lead).push(block);
    }

    this.anchor = chapter.anchor;
    this.title = title ?? chapter.anchor;
    this.lead = lead;
    this.leadText = new BlockText(lead).text;
    this.parts = parts.map((part) => ({ ...part, text: new BlockText(part.blocks).text }));
    this.headingIds = headingIds;
  }

  #getUniqueAnchor(anchor: string, parts: readonly { anchor: string }[]): string {
    let unique = anchor;

    for (let suffix = 2; parts.some((part) => part.anchor === unique); suffix++) unique = `${anchor}-${suffix}`;

    return unique;
  }

  #getText(heading: Block.Heading): string {
    return heading.spans.map((span) => span.text).join('');
  }

  #toSlug(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '');
  }
}

export const ChapterView = memo(function ChapterView({
  structure,
  isHidden,
  hiddenParts
}: {
  structure: ChapterStructure;
  isHidden: boolean;
  hiddenParts: ReadonlySet<string>;
}) {
  return (
    <section class="chapter" hidden={isHidden}>
      <Description blocks={structure.lead} headingIds={structure.headingIds} />
      {structure.parts.map((part) => (
        <section key={part.anchor} class="part" hidden={hiddenParts.has(part.anchor)}>
          <Description blocks={part.blocks} headingIds={structure.headingIds} />
        </section>
      ))}
    </section>
  );
});
