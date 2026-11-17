import { StepMode, type SourceLocation, type StackFrame, type Variable } from 'greybel-runtime';
import { useEffect, useState } from 'preact/hooks';

import { useServices } from './services.js';
import type { RuntimeSession } from '../runtime/session.js';

export interface DebugPanelConfiguration {
  readonly session: RuntimeSession;
  /** Incremented on each stop to reload the stack and scopes. */
  readonly stop: number;
  readonly onResume: () => void;
  readonly onOpenFile: (path: string) => void;
}

interface ScopeContent {
  readonly name: string;
  readonly variables: readonly Variable[];
}

export function DebugPanel({ session, stop, onResume, onOpenFile }: DebugPanelConfiguration) {
  const [frames, setFrames] = useState<readonly StackFrame[]>([]);
  const [scopes, setScopes] = useState<readonly ScopeContent[]>([]);
  const [code, setCode] = useState('');
  const [log, setLog] = useState<readonly string[]>([]);

  useEffect(() => {
    let isCurrent = true;

    void (async () => {
      try {
        const stack = await session.getStackTrace();
        const frame = stack[0];
        const content =
          frame === undefined
            ? []
            : await Promise.all(
                (await session.getScopes(frame.id)).map(async (scope) => ({
                  name: scope.name,
                  variables: await session.getVariables(scope.reference)
                }))
              );

        if (!isCurrent) return;
        setFrames(stack);
        setScopes(content);
      } catch (error) {
        if (isCurrent) setLog((entries) => [...entries, session.describeFailure(error)]);
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
      setLog((entries) => [...entries, `> ${entry}`, session.describeFailure(error)]);
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
              {frame.location && <FrameLocation session={session} location={frame.location} onOpenFile={onOpenFile} />}
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

interface FrameLocationConfiguration {
  readonly session: RuntimeSession;
  readonly location: SourceLocation;
  readonly onOpenFile: (path: string) => void;
}

function FrameLocation({ session, location, onOpenFile }: FrameLocationConfiguration) {
  const path = useServices().paths.toPath(location.uri);

  if (path === undefined) return <span class="debug-location"> {session.describeLocation(location)}</span>;

  return (
    <button type="button" class="debug-location" onClick={() => onOpenFile(path)}>
      {session.describeLocation(location)}
    </button>
  );
}
