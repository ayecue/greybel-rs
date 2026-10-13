import { memo } from 'preact/compat';

import { Block } from '../protocol.generated.js';
import { blockText } from '../state/block-text.js';
import type { RenderedChapter } from '../state/use-guide.js';
import { Description } from './description.js';

/** A part of a chapter: one of its level-2 headings and the blocks up to the next. */
export interface ChapterPart {
  readonly anchor: string;
  readonly title: string;
  readonly blocks: readonly Block[];
  /** The part's text, in lowercase, as the guide's search reads it. */
  readonly text: string;
}

/**
 * A chapter as the page and the sidebar show it: its title, what comes before its first part, its parts, and the ids
 * of their headings.
 */
export interface ChapterOutline {
  readonly anchor: string;
  readonly title: string;
  readonly lead: readonly Block[];
  /** The title and the lead's text, in lowercase, as the guide's search reads them. */
  readonly leadText: string;
  readonly parts: readonly ChapterPart[];
  readonly headingIds: ReadonlyMap<Block, string>;
}

export const ChapterView = memo(function ChapterView({
  outline,
  isHidden,
  hiddenParts
}: {
  outline: ChapterOutline;
  isHidden: boolean;
  hiddenParts: ReadonlySet<string>;
}) {
  return (
    <section class="chapter" hidden={isHidden}>
      <Description blocks={outline.lead} headingIds={outline.headingIds} />
      {outline.parts.map((part) => (
        <section key={part.anchor} class="part" hidden={hiddenParts.has(part.anchor)}>
          <Description blocks={part.blocks} headingIds={outline.headingIds} />
        </section>
      ))}
    </section>
  );
});

/**
 * The outline of `chapter`: its level-1 heading is its title, with the chapter's anchor, and each level-2 heading
 * starts a part `<anchor>-<title>`.
 */
export function outlineOf(chapter: RenderedChapter): ChapterOutline {
  const headingIds = new Map<Block, string>();
  const lead: Block[] = [];
  const parts: { anchor: string; title: string; blocks: Block[] }[] = [];
  let title: string | undefined;

  for (const block of chapter.blocks) {
    if (block instanceof Block.Heading && block.level === 1 && title === undefined) {
      title = textOf(block);
      headingIds.set(block, chapter.anchor);
    } else if (block instanceof Block.Heading && block.level === 2) {
      const text = textOf(block);
      const anchor = uniqueAnchor(`${chapter.anchor}-${slugOf(text)}`, parts);

      headingIds.set(block, anchor);
      parts.push({ anchor, title: text, blocks: [] });
    }

    (parts.at(-1)?.blocks ?? lead).push(block);
  }

  return {
    anchor: chapter.anchor,
    title: title ?? chapter.anchor,
    lead,
    leadText: blockText(lead),
    parts: parts.map((part) => ({ ...part, text: blockText(part.blocks) })),
    headingIds
  };
}

function uniqueAnchor(anchor: string, parts: readonly { anchor: string }[]): string {
  let unique = anchor;

  for (let suffix = 2; parts.some((part) => part.anchor === unique); suffix++) unique = `${anchor}-${suffix}`;

  return unique;
}

function textOf(heading: Block.Heading): string {
  return heading.spans.map((span) => span.text).join('');
}

function slugOf(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}
