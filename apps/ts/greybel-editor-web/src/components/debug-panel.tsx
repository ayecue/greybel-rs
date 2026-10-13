import { StepMode, type SourceLocation, type StackFrame, type Variable } from 'greybel-runtime';
import { useEffect, useState } from 'preact/hooks';

import { filePath } from '../project/paths.js';
import { describeFailure, describeLocation, type RuntimeSession } from '../runtime/session.js';

export interface DebugPanelConfiguration {
  readonly session: RuntimeSession;
  /** Changes each time the program stops, so the panel reads it again. */
  readonly stop: number;
  readonly onResume: () => void;
  /** Shows the project's file at `path`. */
  readonly onOpenFile: (path: string) => void;
}

interface ScopeContent {
  readonly name: string;
  readonly variables: readonly Variable[];
}

/** The stopped program: its call stack, the variables of its innermost frame, and code run where it stopped. */
export function DebugPanel({ session, stop, onResume, onOpenFile }: DebugPanelConfiguration) {
  const [frames, setFrames] = useState<readonly StackFrame[]>([]);
  const [scopes, setScopes] = useState<readonly ScopeContent[]>([]);
  const [code, setCode] = useState('');
  const [log, setLog] = useState<readonly string[]>([]);

  useEffect(() => {
    let isCurrent = true;

    void (async () => {
      try {
        const stack = await session.stackTrace();
        const frame = stack[0];
        const content =
          frame === undefined
            ? []
            : await Promise.all(
                (await session.scopes(frame.id)).map(async (scope) => ({
                  name: scope.name,
                  variables: await session.variables(scope.reference)
                }))
              );

        if (!isCurrent) return;
        setFrames(stack);
        setScopes(content);
      } catch (error) {
        if (isCurrent) setLog((entries) => [...entries, describeFailure(error)]);
      }
    })();

    return () => {
      isCurrent = false;
    };
  }, [session, stop]);

  const evaluate = async () => {
    const entry = code.trim();

    if (entry === '') return;
    setCode('');
    try {
      const evaluation = await session.evaluate(entry);

      setLog((entries) => [...entries, `> ${entry}`, ...(evaluation ? [evaluation.value] : [])]);
    } catch (error) {
      setLog((entries) => [...entries, `> ${entry}`, describeFailure(error)]);
    }
  };

  const step = (mode: StepMode) => {
    onResume();
    session.step(mode);
  };

  return (
    <div class="debug-panel">
      <div class="debug-actions">
        <button
          type="button"
          class="debug-continue"
          onClick={() => {
            onResume();
            session.resume();
          }}
        >
          Continue
        </button>
        <button type="button" class="debug-step-over" onClick={() => step(StepMode.Over)}>
          Step over
        </button>
        <button type="button" class="debug-step-into" onClick={() => step(StepMode.Into)}>
          Step into
        </button>
        <button type="button" class="debug-step-out" onClick={() => step(StepMode.Out)}>
          Step out
        </button>
      </div>
      <div class="debug-columns">
        <ol class="debug-stack">
          {frames.map((frame) => (
            <li key={frame.id}>
              {frame.name}
              {frame.location && <FrameLocation location={frame.location} onOpenFile={onOpenFile} />}
            </li>
          ))}
        </ol>
        <div class="debug-scopes">
          {scopes.map((scope) => (
            <details key={scope.name} open={scope.name !== 'Globals'}>
              <summary>{scope.name}</summary>
              <ul>
                {scope.variables.map((variable) => (
                  <li key={variable.name}>
                    <span class="debug-name">{variable.name}</span> = <span>{variable.value}</span>
                  </li>
                ))}
              </ul>
            </details>
          ))}
        </div>
      </div>
      <pre class="debug-log">{log.join('\n')}</pre>
      <input
        type="text"
        class="debug-evaluate"
        placeholder="Run code where the program stopped"
        value={code}
        onInput={(event) => setCode(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key !== 'Enter') return;
          void evaluate();
        }}
      />
    </div>
  );
}

/** Where a frame is: a button showing its file, when it is a file of the project. */
function FrameLocation({ location, onOpenFile }: { location: SourceLocation; onOpenFile: (path: string) => void }) {
  const path = filePath(location.uri);

  if (path === undefined) return <span class="debug-location"> {describeLocation(location)}</span>;

  return (
    <button type="button" class="debug-location" onClick={() => onOpenFile(path)}>
      {describeLocation(location)}
    </button>
  );
}
