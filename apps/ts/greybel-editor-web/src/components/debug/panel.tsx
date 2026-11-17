import { StepMode, type StackFrame, type Variable } from 'greybel-runtime';
import { useEffect, useReducer, useState } from 'preact/hooks';

import type { ProgramTerminal } from '../../runtime/program-terminal.js';
import { Splitter } from '../splitter.js';
import { CallStack } from './call-stack.js';
import { DebugConsole } from './console.js';
import { ValueNode } from './value-node.js';
import { WatchView } from './watch-view.js';

interface ScopeContent {
  readonly name: string;
  readonly reference: number;
  readonly variables: readonly Variable[];
}

export interface DebugPanelConfiguration {
  readonly terminal: ProgramTerminal;
  readonly onResume: () => void;
  readonly onStop: () => void;
}

/** Shown during a debug run: the stop, the call stack, variables, watch expressions and the console. */
export function DebugPanel({ terminal, onResume, onStop }: DebugPanelConfiguration) {
  const { session, debug } = terminal;
  const isStopped = terminal.state === 'stopped';
  const [, refresh] = useReducer((count: number) => count + 1, 0);
  const [frames, setFrames] = useState<readonly StackFrame[]>([]);
  const [frameIndex, setFrameIndex] = useState(0);
  const [scopes, setScopes] = useState<readonly ScopeContent[]>([]);
  const [problem, setProblem] = useState<string>();

  useEffect(() => {
    if (!isStopped) return;

    let isLatest = true;

    setFrameIndex(0);
    session.getStackTrace().then(
      (stack) => {
        if (!isLatest) return;
        setFrames(stack);
      },
      (error: unknown) => {
        if (!isLatest) return;
        setProblem(session.describeFailure(error));
      }
    );

    return () => {
      isLatest = false;
    };
  }, [session, isStopped, debug.stop]);

  const frame = frames[frameIndex];

  useEffect(() => {
    if (!isStopped || frame === undefined) return;

    let isLatest = true;

    void (async () => {
      try {
        const content = await Promise.all(
          (await session.getScopes(frame.id)).map(async (scope) => ({
            name: scope.name,
            reference: scope.reference,
            variables: await session.getVariables(scope.reference)
          }))
        );

        if (isLatest) {
          setScopes(content);
          setProblem(undefined);
        }
      } catch (error) {
        if (!isLatest) return;
        setProblem(session.describeFailure(error));
      }
    })();

    return () => {
      isLatest = false;
    };
  }, [session, isStopped, frame]);

  const selectFrame = (index: number) => {
    setFrameIndex(index);
    terminal.selectFrame(index === 0 ? undefined : (frames[index]?.location ?? undefined));
  };

  const step = (mode: StepMode) => {
    onResume();
    session.step(mode);
  };

  const status = debug.describeStop((location) => session.describeLocation(location)) ?? 'Running';

  return (
    <div class={`debug-panel${isStopped ? ' stopped' : ' running'}`}>
      <div class="debug-toolbar">
        <span class="debug-status">{status}</span>
        <button
          type="button"
          class="debug-continue"
          disabled={!isStopped}
          onClick={() => {
            onResume();
            session.resume();
          }}
        >
          Continue
        </button>
        <button type="button" class="debug-step-over" disabled={!isStopped} onClick={() => step(StepMode.Over)}>
          Step over
        </button>
        <button type="button" class="debug-step-into" disabled={!isStopped} onClick={() => step(StepMode.Into)}>
          Step into
        </button>
        <button type="button" class="debug-step-out" disabled={!isStopped} onClick={() => step(StepMode.Out)}>
          Step out
        </button>
        <button type="button" class="debug-pause" disabled={isStopped} onClick={() => session.pause()}>
          Pause
        </button>
        <button type="button" class="debug-stop" onClick={onStop}>
          Stop
        </button>
      </div>
      {problem !== undefined && <div class="debug-problem">{problem}</div>}
      <div class="debug-sections">
        <section class="debug-section debug-stack-section">
          <h3>Call stack</h3>
          <CallStack session={session} frames={frames} frameIndex={frameIndex} onSelect={selectFrame} />
        </section>
        <Splitter name="debug-stack" axis="x" side="before" min={120} />
        <section class="debug-section debug-variables-section">
          <h3>Variables</h3>
          {scopes.map((scope) => (
            <details key={scope.name} class="debug-scope" open={scope.name !== 'Globals'}>
              <summary>{scope.name}</summary>
              <ul class="value-tree">
                {scope.variables.map((variable) => (
                  <ValueNode
                    key={variable.name}
                    session={session}
                    debug={debug}
                    row={variable}
                    path={`${scope.name}/${variable.name}`}
                    isCurrent={isStopped}
                    parentReference={scope.reference}
                  />
                ))}
              </ul>
            </details>
          ))}
        </section>
        <Splitter name="debug-watch" axis="x" side="after" min={120} />
        <section class="debug-section debug-watch-section">
          <h3>Watch</h3>
          <WatchView session={session} debug={debug} isStopped={isStopped} />
        </section>
      </div>
      <DebugConsole session={session} debug={debug} isStopped={isStopped} onChange={() => refresh(undefined)} />
    </div>
  );
}
