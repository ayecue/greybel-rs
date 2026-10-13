/** A search field with a button that clears it, shown whenever it has text. */
export function SearchBox({
  query,
  placeholder,
  onQuery
}: {
  query: string;
  placeholder: string;
  onQuery: (query: string) => void;
}) {
  return (
    <div class="search-box">
      <input
        class="search"
        type="search"
        placeholder={placeholder}
        aria-label={placeholder}
        value={query}
        onInput={(event) => onQuery(event.currentTarget.value)}
      />
      {query !== '' && (
        <button
          type="button"
          class="clear"
          title="Clear the search"
          aria-label="Clear the search"
          onClick={() => onQuery('')}
        />
      )}
    </div>
  );
}
