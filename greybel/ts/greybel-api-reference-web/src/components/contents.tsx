import { Link, type Entry } from '../protocol.generated.js';
import type { GuideVisibility } from '../state/guide-visibility.js';
import type { Visibility } from '../state/visibility.js';
import type { ChapterOutline } from './chapter.js';
import { LinkTo } from './link.js';

/** The guide's sidebar: each chapter with its parts, as far as the search shows them. */
export function GuideContents({
  outlines,
  visibility
}: {
  outlines: readonly ChapterOutline[];
  visibility: GuideVisibility;
}) {
  return (
    <nav class="contents" aria-label="Chapters">
      <ul>
        {outlines.map(({ anchor, title, parts }) => (
          <li hidden={visibility.hiddenChapters.has(anchor)}>
            <LinkTo link={new Link.Page({ anchor })} className="entry">
              {title}
            </LinkTo>
            <ul>
              {parts.map((part) => (
                <li hidden={visibility.hiddenParts.has(part.anchor)}>
                  <LinkTo link={new Link.Page({ anchor: part.anchor })}>{part.title}</LinkTo>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </nav>
  );
}

/** The API's sidebar: each type with its members, as far as the search and the tags show them. */
export function ApiContents({ entries, visibility }: { entries: readonly Entry[]; visibility: Visibility }) {
  return (
    <nav class="contents" aria-label="Types">
      <ul>
        {entries.map((entry) => (
          <li hidden={visibility.hiddenTypes.has(entry.name)}>
            <LinkTo link={new Link.Type({ name: entry.name })} className="entry">
              {entry.name}
            </LinkTo>
            <ul>
              {entry.members.map((name) => (
                <li hidden={visibility.hiddenMembers.get(entry.name)?.has(name) ?? false}>
                  <LinkTo link={new Link.Member({ owner: entry.name, name })}>{name}</LinkTo>
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </nav>
  );
}
