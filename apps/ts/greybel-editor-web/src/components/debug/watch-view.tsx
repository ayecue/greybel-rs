import type { Evaluation } from 'greybel-runtime';
import { useEffect, useReducer, useState } from 'preact/hooks';

import type { DebugState } from '../../runtime/debug-state.js';
import type { RuntimeSession } from '../../runtime/session.js';
import { useServices } from '../services.js';
import { ValueNode } from './value-node.js';

type WatchResult = { readonly evaluation: Evaluation | null } | { readonly problem: string };

export interface WatchViewConfiguration {
  readonly session: RuntimeSession;
  readonly debug: DebugState;
  readonly isStopped: boolean;
}

/** Watch expressions, evaluated again at each stop. */
export function WatchView({ session, debug, isStopped }: WatchViewConfiguration) {
  const { watches } = useServices();
  const [, refresh] = useReducer((count: number) => count + 1, 0);
  const [results, setResults] = useState<ReadonlyMap<string, WatchResult>>(new Map());
  const [draft, setDraft] = useState('');
  const { expressions } = watches;

  useEffect(() => watches.subscribe(() => refresh(undefined)), [watches]);

  useEffect(() => {
    if (!isStopped) return;

    let isLatest = true;

    void Promise.all(
      expressions.map(async (expression): Promise<[string, WatchResult]> => {
        try {
          return [expression, { evaluation: await session.evaluate(expression) }];
        } catch (error) {
          return [expression, { problem: session.describeFailure(error) }];
        }
      })
    ).then((entries) => {
      if (!isLatest) return;
      setResults(new Map(entries));
    });

    return () => {
      isLatest = false;
    };
  }, [session, isStopped, debug.stop, expressions]);

  return (
    <div class="watch-view">
      <ul class="value-tree">
        {expressions.map((expression) => {
          const result = results.get(expression);

          return (
            <li key={expression} class="watch-entry">
              <button
                type="button"
                class="watch-remove"
                title="Removes the expression"
                onClick={() => watches.remove(expression)}
              >
                ×
              </button>
              {result === undefined || !('evaluation' in result) ? (
                <div class="value-row">
                  <span class="value-name">{expression}</span>
                  <span class={result === undefined ? 'value-text' : 'value-problem'}>
                    {result === undefined ? '' : result.problem}
                  </span>
                </div>
              ) : (
                <ul class="value-tree">
                  <ValueNode
                    session={session}
                    debug={debug}
                    row={{
                      name: expression,
                      value: result.evaluation?.value ?? 'null',
                      typeName: result.evaluation?.typeName ?? '',
                      reference: result.evaluation?.reference ?? 0
                    }}
                    path={`watch/${expression}`}
                    isCurrent={isStopped}
                  />
                </ul>
              )}
            </li>
          );
        })}
      </ul>
      <input
        type="text"
        class="watch-add"
        placeholder="Add an expression"
        value={draft}
        onInput={(event) => setDraft(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key !== 'Enter') return;
          watches.add(draft);
          setDraft('');
        }}
      />
    </div>
  );
}
