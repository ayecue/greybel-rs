import type { ChapterStructure } from '../components/chapter.js';

/** Guide chapters and sections hidden by search, keyed by anchor. */
export interface GuideVisibility {
  readonly hiddenChapters: ReadonlySet<string>;
  readonly hiddenParts: ReadonlySet<string>;
}

export const GUIDE_VISIBLE: GuideVisibility = { hiddenChapters: new Set(), hiddenParts: new Set() };

/** Chapter structures whose visibility depends on a guide search query. */
export class GuideSearch {
  readonly #structures: readonly ChapterStructure[];

  constructor(structures: readonly ChapterStructure[]) {
    this.#structures = structures;
  }

  /**
   * A section matches when its text or chapter title contains every query word case-insensitively. A chapter is fully
   * visible when its title and introduction match; otherwise only matching sections remain visible.
   */
  getVisibility(query: string): GuideVisibility {
    const words = query
      .toLowerCase()
      .split(/\s+/)
      .filter((word) => word !== '');

    if (words.length === 0) return GUIDE_VISIBLE;

    const hiddenChapters = new Set<string>();
    const hiddenParts = new Set<string>();
    const containsAll = (text: string) => words.every((word) => text.includes(word));

    for (const structure of this.#structures) {
      if (containsAll(structure.leadText)) continue;

      const title = structure.title.toLowerCase();
      const partsWithoutMatch = structure.parts.filter((part) => !containsAll(`${title}\n${part.text}`));

      for (const part of partsWithoutMatch) hiddenParts.add(part.anchor);
      if (partsWithoutMatch.length !== structure.parts.length) continue;
      hiddenChapters.add(structure.anchor);
    }

    return { hiddenChapters, hiddenParts };
  }
}
