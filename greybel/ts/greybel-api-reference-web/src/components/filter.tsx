import { SearchBox } from './search-box.js';

export function Filter({
  query,
  tags,
  excludedTags,
  onQuery,
  onToggleTag
}: {
  query: string;
  tags: readonly string[];
  excludedTags: ReadonlySet<string>;
  onQuery: (query: string) => void;
  onToggleTag: (tag: string) => void;
}) {
  return (
    <div class="filter">
      <SearchBox query={query} placeholder="Search the API" onQuery={onQuery} />
      <div class="tag-filter">
        {tags.map((tag) => (
          <button
            type="button"
            class={excludedTags.has(tag) ? 'tag excluded' : 'tag'}
            title={`Hide the members tagged ${tag}`}
            onClick={() => onToggleTag(tag)}
          >
            {tag}
          </button>
        ))}
      </div>
    </div>
  );
}
