import { useEffect, useRef, useState } from 'preact/hooks';

import type { DebugState } from '../../runtime/debug-state.js';
import type { RuntimeSession } from '../../runtime/session.js';
import { ValueNode } from './value-node.js';

export interface DebugConsoleConfiguration {
  readonly session: RuntimeSession;
  readonly debug: DebugState;
  readonly isStopped: boolean;
  readonly onChange: () => void;
}

/** Runs code where the program stopped; its entries last for the entire debug run. */
export function DebugConsole({ session, debug, isStopped, onChange }: DebugConsoleConfiguration) {
  const [code, setCode] = useState('');
  /** Position in `debug.history` while browsing it; undefined while editing a new input. */
  const [historyIndex, setHistoryIndex] = useState<number>();
  const logRef = useRef<HTMLDivElement>(null);
  const { entries } = debug;

  useEffect(() => {
    const log = logRef.current;

    if (log === null) return;
    log.scrollTop = log.scrollHeight;
  }, [entries]);

  const evaluate = async () => {
    const entry = code.trim();

    if (entry === '') return;
    setCode('');
    setHistoryIndex(undefined);
    debug.addInput(entry);
    onChange();
    try {
      const evaluation = await session.evaluate(entry);

      if (evaluation) debug.addEntry('output', evaluation.value, evaluation);
    } catch (error) {
      debug.addEntry('error', session.describeFailure(error));
    }

    onChange();
  };

  const browse = (step: number) => {
    const { history } = debug;

    if (history.length === 0) return;

    const index = Math.min(Math.max((historyIndex ?? history.length) + step, 0), history.length);

    setHistoryIndex(index === history.length ? undefined : index);
    setCode(history[index] ?? '');
  };

  return (
    <div class="debug-console">
      <div class="debug-log" ref={logRef}>
        {entries.map((entry, index) => (
          <div key={index} class={`console-entry ${entry.kind}`}>
            {entry.evaluation !== undefined && entry.evaluation.reference > 0 ? (
              <ul class="value-tree">
                <ValueNode
                  session={session}
                  debug={debug}
                  row={{
                    name: '',
                    value: entry.evaluation.value,
                    typeName: entry.evaluation.typeName,
                    reference: entry.evaluation.reference
                  }}
                  path={`console/${index}`}
                  isCurrent={isStopped && entry.stop === debug.stop}
                />
              </ul>
            ) : (
              <>{entry.kind === 'input' ? `> ${entry.text}` : entry.text}</>
            )}
          </div>
        ))}
      </div>
      <input
        type="text"
        class="debug-evaluate"
        placeholder={isStopped ? 'Run code where the program stopped' : 'Pause the program to run code'}
        value={code}
        onInput={(event) => {
          setCode(event.currentTarget.value);
          setHistoryIndex(undefined);
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') void evaluate();
          else if (event.key === 'ArrowUp') {
            event.preventDefault();
            browse(-1);
          } else if (event.key === 'ArrowDown') {
            event.preventDefault();
            browse(1);
          }
        }}
      />
    </div>
  );
}
