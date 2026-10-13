import { useEffect, useState } from 'preact/hooks';

import { Search, type SearchResult } from '../protocol.generated.js';
import type { ReaderChannel } from '../reader.generated.js';

const SEARCH_DELAY = 100;

export function useSearch(
  reader: ReaderChannel,
  query: string,
  excludedTags: ReadonlySet<string>
): SearchResult | undefined {
  const [result, setResult] = useState<SearchResult>();

  useEffect(() => {
    const controller = new AbortController();
    const timer = setTimeout(() => {
      reader
        .send(new Search({ query, excludedTags: [...excludedTags] }), { signal: controller.signal })
        .then(setResult, (error: unknown) => {
          if (!controller.signal.aborted) throw error;
        });
    }, SEARCH_DELAY);

    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  }, [reader, query, excludedTags]);

  return result;
}
