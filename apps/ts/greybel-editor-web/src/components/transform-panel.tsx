import { ChannelError } from 'duplex-core';
import { BuildFailure } from 'greybel-builder';
import { useState } from 'preact/hooks';

import type { Builder, TransformModeName } from '../builder.js';

export interface TransformPanelConfiguration {
  readonly builder: Builder;
  /** The code to transform: the file shown's, none when no file is. */
  readonly code: () => string | undefined;
  readonly onFailure: (message: string) => void;
}

/** Transforms the document on its own: as written, uglified or beautified, the result shown to copy. */
export function TransformPanel({ builder, code, onFailure }: TransformPanelConfiguration) {
  const [mode, setMode] = useState<TransformModeName>('uglify');
  const [obfuscation, setObfuscation] = useState(false);
  const [disableLiterals, setDisableLiterals] = useState(false);
  const [disableNamespaces, setDisableNamespaces] = useState(false);
  const [retainParentheses, setRetainParentheses] = useState(false);
  const [useTabs, setUseTabs] = useState(true);
  const [spaces, setSpaces] = useState(2);
  const [excludedNames, setExcludedNames] = useState('');
  const [result, setResult] = useState('');
  const [hasTransformRequest, setTransformRequest] = useState(false);

  const transform = async () => {
    const source = code();

    if (source === undefined) {
      onFailure('Open a file to transform it.');

      return;
    }

    setTransformRequest(true);
    try {
      setResult(
        await builder.transform(source, {
          mode,
          obfuscation,
          disableLiteralsOptimization: disableLiterals,
          disableNamespacesOptimization: disableNamespaces,
          retainParentheses,
          indentationSpaces: useTabs ? undefined : spaces,
          excludedNames: excludedNames
            .split(',')
            .map((name) => name.trim())
            .filter((name) => name !== '')
        })
      );
    } catch (error) {
      onFailure(transformFailure(error));
    } finally {
      setTransformRequest(false);
    }
  };

  return (
    <section class="panel transform-panel">
      <header class="panel-header">
        <h2>Transform</h2>
        <select
          class="transform-mode"
          value={mode}
          onChange={(event) => setMode(event.currentTarget.value as TransformModeName)}
        >
          <option value="default">Default</option>
          <option value="uglify">Uglify</option>
          <option value="beautify">Beautify</option>
        </select>
        <button type="button" class="transform" disabled={hasTransformRequest} onClick={() => void transform()}>
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
      </header>
      <div class="panel-options">
        {mode === 'uglify' && (
          <>
            <label>
              <input
                type="checkbox"
                checked={obfuscation}
                onChange={(event) => setObfuscation(event.currentTarget.checked)}
              />
              Obfuscation
            </label>
            <label>
              <input
                type="checkbox"
                checked={disableLiterals}
                onChange={(event) => setDisableLiterals(event.currentTarget.checked)}
              />
              No literals optimization
            </label>
            <label>
              <input
                type="checkbox"
                checked={disableNamespaces}
                onChange={(event) => setDisableNamespaces(event.currentTarget.checked)}
              />
              No namespaces optimization
            </label>
            <label>
              Retained names
              <input
                type="text"
                placeholder="name, other"
                value={excludedNames}
                onInput={(event) => setExcludedNames(event.currentTarget.value)}
              />
            </label>
          </>
        )}
        {mode === 'beautify' && (
          <>
            <label>
              <input
                type="checkbox"
                checked={retainParentheses}
                onChange={(event) => setRetainParentheses(event.currentTarget.checked)}
              />
              Retain parentheses
            </label>
            <label>
              <input type="checkbox" checked={useTabs} onChange={(event) => setUseTabs(event.currentTarget.checked)} />
              Indent with tabs
            </label>
            {!useTabs && (
              <label>
                Spaces
                <input
                  type="number"
                  min={1}
                  max={8}
                  value={spaces}
                  onInput={(event) => setSpaces(Number(event.currentTarget.value) || 2)}
                />
              </label>
            )}
          </>
        )}
      </div>
      <textarea class="transform-result" readOnly value={result} spellcheck={false} />
    </section>
  );
}

function transformFailure(error: unknown): string {
  const failure = error instanceof ChannelError ? (error.failure as unknown) : undefined;

  if (failure instanceof BuildFailure.Invalid) {
    const at = failure.location ? ` at line ${failure.location.line}, character ${failure.location.character}` : '';

    return `Transform error: ${failure.message}${at}`;
  }

  return error instanceof Error ? error.message : String(error);
}
