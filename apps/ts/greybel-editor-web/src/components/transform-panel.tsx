import { useEffect, useReducer, useState } from 'preact/hooks';

import type { Builder } from '../builder.js';
import type { ProjectEditor } from '../editor/view.js';
import { useServices } from './services.js';
import { usePartChanges } from './wasm-status.js';
import { MANIFEST_PATH } from '../project/content.js';
import type { ProjectSettings } from '../project/settings.js';

export interface TransformPanelConfiguration {
  readonly builder: Builder;
  readonly editor: ProjectEditor;
  readonly settings: ProjectSettings;
  readonly onFailure: (message: string) => void;
}

export function TransformPanel({ builder, editor, settings, onFailure }: TransformPanelConfiguration) {
  const [result, setResult] = useState('');
  const [hasTransformRequest, setTransformRequest] = useState(false);
  const [, refresh] = useReducer((count: number) => count + 1, 0);

  const isTransformerReady = useServices().wasm.isReady('language server', 'transformer');

  usePartChanges();
  useEffect(() => settings.subscribe(() => refresh(undefined)), [settings]);

  const transform = async () => {
    const path = editor.path;
    const code = editor.code;

    if (path === undefined || code === undefined || path === MANIFEST_PATH) {
      onFailure('Open a script to transform it.');

      return;
    }

    setTransformRequest(true);
    try {
      setResult(await builder.transform(code, await settings.read(path)));
    } catch (error) {
      onFailure(builder.describeFailure(error));
    } finally {
      setTransformRequest(false);
    }
  };

  return (
    <section class="panel transform-panel">
      <header class="panel-header">
        <h2>Transform</h2>
        <span class="transform-mode" title="build.mode in greybel.json">
          {settings.current?.manifest.build.mode}
        </span>
        <button
          type="button"
          class="transform"
          disabled={hasTransformRequest || !isTransformerReady}
          title={isTransformerReady ? undefined : 'Loading the transformer...'}
          onClick={() => void transform()}
        >
          Transform
        </button>
        <button
          type="button"
          disabled={result === ''}
          onClick={() => void navigator.clipboard.writeText(result)}
          title="Copy the transformed code"
        >
          Copy
        </button>
        <button type="button" class="open-settings" onClick={() => editor.open(MANIFEST_PATH)}>
          greybel.json
        </button>
      </header>
      <textarea class="transform-result" readOnly value={result} spellcheck={false} />
    </section>
  );
}
