import type { Overview, SearchResult } from '../protocol.generated.js';

export interface Visibility {
  readonly hiddenTypes: ReadonlySet<string>;
  readonly hiddenMembers: ReadonlyMap<string, ReadonlySet<string>>;
}

export const ALL_VISIBLE: Visibility = { hiddenTypes: new Set(), hiddenMembers: new Map() };

/** API contents whose visibility depends on a search result. */
export class ApiSearch {
  readonly #overview: Overview;

  constructor(overview: Overview) {
    this.#overview = overview;
  }

  getVisibility(result: SearchResult): Visibility {
    const typeMatches = new Set(result.types);
    const memberMatches = new Map<string, Set<string>>();

    for (const { owner, name } of result.members) {
      let names = memberMatches.get(owner);

      if (names === undefined) memberMatches.set(owner, (names = new Set()));
      names.add(name);
    }

    const hiddenTypes = new Set<string>();
    const hiddenMembers = new Map<string, Set<string>>();

    for (const entry of this.#overview.contents) {
      const entryMatches = memberMatches.get(entry.name);

      hiddenMembers.set(entry.name, new Set(entry.members.filter((name) => !(entryMatches?.has(name) ?? false))));
      if (typeMatches.has(entry.name) || entryMatches !== undefined) continue;
      hiddenTypes.add(entry.name);
    }

    return { hiddenTypes, hiddenMembers };
  }
}
