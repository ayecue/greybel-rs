import type { ChapterOutline } from '../components/chapter.js';

/** Guide chapters and sections hidden by search, keyed by anchor. */
export interface GuideVisibility {
  readonly hiddenChapters: ReadonlySet<string>;
  readonly hiddenParts: ReadonlySet<string>;
}

export const GUIDE_VISIBLE: GuideVisibility = { hiddenChapters: new Set(), hiddenParts: new Set() };

/**
 * Computes guide content hidden by `query`. A section matches when its text or chapter title contains every
 * query word case-insensitively. A chapter is fully visible when its title and introduction match; otherwise
 * only matching sections remain visible.
 */
export function guideVisibilityOf(outlines: readonly ChapterOutline[], query: string): GuideVisibility {
  const words = query
    .toLowerCase()
    .split(/\s+/)
    .filter((word) => word !== '');

  if (words.length === 0) return GUIDE_VISIBLE;

  const hiddenChapters = new Set<string>();
  const hiddenParts = new Set<string>();
  const containsAll = (text: string) => words.every((word) => text.includes(word));

  for (const outline of outlines) {
    if (containsAll(outline.leadText)) continue;

    const title = outline.title.toLowerCase();
    const partsWithoutMatch = outline.parts.filter((part) => !containsAll(`${title}\n${part.text}`));

    for (const part of partsWithoutMatch) hiddenParts.add(part.anchor);
    if (partsWithoutMatch.length !== outline.parts.length) continue;
    hiddenChapters.add(outline.anchor);
  }

  return { hiddenChapters, hiddenParts };
}
