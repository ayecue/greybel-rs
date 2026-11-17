import { useEffect, useState } from 'preact/hooks';

import type { Guide } from '../guide.js';
import { Render, type Block } from '../protocol.generated.js';
import type { ReaderChannel } from '../reader.generated.js';

export interface ChapterContent {
  readonly anchor: string;
  readonly blocks: readonly Block[];
}

export interface GuideContent {
  readonly introduction: readonly Block[];
  readonly chapters: readonly ChapterContent[];
}

const NOTHING: GuideContent = { introduction: [], chapters: [] };

/**
 * Renders `guide` introduction and chapter Markdown through the reader and resolves when all rendering
 * completes.
 */
export function useGuide(reader: ReaderChannel, guide: Guide, onFailure?: (error: unknown) => void): GuideContent {
  const [content, setContent] = useState(NOTHING);

  useEffect(() => {
    const controller = new AbortController();
    const render = (markdown: string) => reader.send(new Render({ markdown }), { signal: controller.signal });

    Promise.all([
      guide.introduction === undefined ? [] : render(guide.introduction),
      Promise.all(guide.chapters.map(async ({ anchor, markdown }) => ({ anchor, blocks: await render(markdown) })))
    ]).then(
      ([introduction, chapters]) => setContent({ introduction, chapters }),
      (error: unknown) => {
        if (controller.signal.aborted) return;
        onFailure?.(error);
      }
    );

    return () => controller.abort();
  }, [reader, guide]);

  return content;
}
