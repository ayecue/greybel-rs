import { useEffect, useState } from 'preact/hooks';

import { Reference, type Overview, type Section } from '../protocol.generated.js';
import type { ReaderChannel } from '../reader.generated.js';

export interface ReferenceContent {
  readonly overview?: Overview;
  readonly sections: readonly Section[];
}

export interface ReferenceProgress {
  readonly onComplete?: () => void;
  readonly onFailure?: (error: unknown) => void;
}

export function useReference(reader: ReaderChannel, { onComplete, onFailure }: ReferenceProgress): ReferenceContent {
  const [content, setContent] = useState<ReferenceContent>({ sections: [] });

  useEffect(() => {
    const controller = new AbortController();
    const stream = reader.send(new Reference(), { signal: controller.signal });

    void (async () => {
      try {
        const overview = await stream.start();

        setContent({ overview, sections: [] });
        for await (const section of stream) {
          setContent((current) => ({ ...current, sections: [...current.sections, section] }));
        }

        onComplete?.();
      } catch (error) {
        if (!controller.signal.aborted) onFailure?.(error);
      }
    })();

    return () => controller.abort();
  }, [reader]);

  return content;
}
