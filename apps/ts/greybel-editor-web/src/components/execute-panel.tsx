import { useEffect, useReducer, useRef, useState } from 'preact/hooks';

import type { ProjectEditor } from '../editor/view.js';
import type { Project } from '../project.js';
import { fileUri } from '../project/paths.js';
import { ProgramTerminal } from '../runtime/program-terminal.js';
import { describeExit, describeFailure } from '../runtime/session.js';
import { WorldWorker } from '../runtime/world.js';
import { DebugPanel } from './debug-panel.js';

/** The seed of the world the page starts with. */
const DEFAULT_SEED = 'test';

export interface ExecutePanelConfiguration {
  readonly editor: ProjectEditor;
  readonly project: Project;
  /** Where the editor's breakpoint changes go, once the panel has runtimes. */
  readonly breakpointChanges: { current: ((path: string, lines: number[]) => void) | undefined };
  readonly onFailure: (message: string) => void;
}

/** The page's terminals and the game server whose world their programs share. */
interface TerminalTable {
  readonly world: WorldWorker;
  readonly list: ProgramTerminal[];
  count: number;
}

/**
 * Runs and debugs the project from its entry file, on Grey Hack's terminals: each terminal runs a program of its own,
 * at the same time as the others, all in one simulated world, which lasts from run to run until it is reset.
 */
export function ExecutePanel({ editor, project, breakpointChanges, onFailure }: ExecutePanelConfiguration) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalsRef = useRef<TerminalTable | undefined>(undefined);
  const [current, setCurrent] = useState<ProgramTerminal>();
  const currentRef = useRef<ProgramTerminal | undefined>(undefined);
  const [, refresh] = useReducer((count: number) => count + 1, 0);
  const [params, setParams] = useState('');
  const [seed, setSeed] = useState(DEFAULT_SEED);

  const showStopOf = (terminal: ProgramTerminal | undefined) => editor.showStop(terminal?.stopPlace);

  const select = (terminal: ProgramTerminal | undefined) => {
    currentRef.current = terminal;
    setCurrent(terminal);
    for (const other of terminalsRef.current?.list ?? []) other.canvas.hidden = other !== terminal;
    showStopOf(terminal);
  };

  /** Opens a new terminal and selects it; none when its canvas can't be drawn. */
  const addTerminal = (): ProgramTerminal | undefined => {
    const terminals = terminalsRef.current;
    const container = containerRef.current;

    if (terminals === undefined || container === null) return undefined;

    let terminal: ProgramTerminal;

    try {
      terminal = new ProgramTerminal(`Terminal ${terminals.count + 1}`, terminals.world, {
        onChange: (target) => {
          if (target === currentRef.current && target.state !== 'stopped') showStopOf(target);
          refresh(undefined);
        },
        onStop: (target) => select(target)
      });
    } catch (error) {
      onFailure(`The terminal can't be drawn: ${error instanceof Error ? error.message : String(error)}`);

      return undefined;
    }

    terminals.count += 1;
    terminals.list.push(terminal);
    container.append(terminal.canvas);
    select(terminal);

    return terminal;
  };

  const closeTerminal = (terminal: ProgramTerminal) => {
    const list = terminalsRef.current?.list;

    if (list === undefined || list.length === 1) return;

    const index = list.indexOf(terminal);

    list.splice(index, 1);
    terminal.close();
    if (terminal === currentRef.current) select(list[Math.min(index, list.length - 1)]);
    else refresh(undefined);
  };

  useEffect(() => {
    terminalsRef.current = { world: new WorldWorker(DEFAULT_SEED), list: [], count: 0 };
    addTerminal();
    breakpointChanges.current = (path, lines) => {
      for (const terminal of terminalsRef.current?.list ?? []) {
        void terminal.session.setBreakpoints(fileUri(path), lines).catch(() => undefined);
      }
    };

    return () => {
      breakpointChanges.current = undefined;
      for (const terminal of terminalsRef.current?.list ?? []) terminal.close();
      terminalsRef.current?.world.close();
      terminalsRef.current = undefined;
    };
  }, [editor]);

  const run = async (debug: boolean) => {
    const terminal = currentRef.current;
    const world = terminalsRef.current?.world;

    if (terminal === undefined || world === undefined || terminal.state !== 'idle') return;

    const session = terminal.session;

    terminal.update('running');
    try {
      for (const [uri, lines] of editor.breakpoints()) await session.setBreakpoints(uri, lines);

      const summary = describeExit(
        await session.run({
          documents: project.documents,
          entry: project.entry,
          params: params.split(' ').filter((param) => param !== ''),
          seed: world.seed,
          debug
        })
      );

      if (summary !== undefined) terminal.terminal.printLine(summary);
    } catch (error) {
      if (session === terminal.session) terminal.terminal.printLine(describeFailure(error));
    } finally {
      if (session === terminal.session) {
        terminal.terminal.cancelRead();
        terminal.update('idle');
      }
    }
  };

  /** Replaces the world with one generated from the seed, stopping the programs running in the old one. */
  const resetWorld = () => {
    const terminals = terminalsRef.current;

    if (terminals === undefined) return;
    terminals.world.reset(seed);
    for (const terminal of terminals.list) {
      if (terminal.state !== 'idle') terminal.terminal.printLine('The program was stopped: the world was reset.');
      terminal.reconnect(terminals.world);
    }

    showStopOf(currentRef.current);
  };

  const resume = () => {
    current?.update('running');
  };

  const state = current?.state ?? 'idle';
  const list = terminalsRef.current?.list ?? [];

  return (
    <section class="panel execute-panel">
      <header class="panel-header">
        <h2>Execute</h2>
        <button type="button" class="run" disabled={state !== 'idle'} onClick={() => void run(false)}>
          Run
        </button>
        <button type="button" class="debug" disabled={state !== 'idle'} onClick={() => void run(true)}>
          Debug
        </button>
        <button
          type="button"
          class="pause"
          disabled={state !== 'running'}
          onClick={() => current?.session.pause()}
          title="Stops the program at its next statement, when it runs in debug mode"
        >
          Pause
        </button>
        <button
          type="button"
          class="stop"
          disabled={state === 'idle'}
          onClick={() => {
            current?.terminal.cancelRead();
            current?.session.terminate();
          }}
        >
          Stop
        </button>
        <button type="button" class="clear" onClick={() => current?.terminal.clear()}>
          Clear
        </button>
      </header>
      <div class="panel-options">
        <label>
          Params
          <input
            type="text"
            class="params"
            value={params}
            placeholder="separated by spaces"
            onInput={(event) => setParams(event.currentTarget.value)}
          />
        </label>
        <label>
          Seed
          <input type="text" class="seed" value={seed} onInput={(event) => setSeed(event.currentTarget.value)} />
        </label>
        <button
          type="button"
          class="reset-world"
          onClick={resetWorld}
          title="Generates a new world from the seed; the programs running in the old one stop"
        >
          New world
        </button>
      </div>
      <div class="terminal-tabs" role="tablist">
        {list.map((terminal) => (
          <div
            key={terminal.name}
            class={`terminal-tab${terminal === current ? ' active' : ''}`}
            role="tab"
            aria-selected={terminal === current}
            data-state={terminal.state}
            onClick={() => select(terminal)}
          >
            <span class="terminal-name">{terminal.name}</span>
            {terminal.state !== 'idle' && <span class="terminal-state">{terminal.state}</span>}
            {list.length > 1 && (
              <button
                type="button"
                class="terminal-close"
                title="Closes the terminal, stopping its program"
                onClick={(event) => {
                  event.stopPropagation();
                  closeTerminal(terminal);
                }}
              >
                ×
              </button>
            )}
          </div>
        ))}
        <button
          type="button"
          class="terminal-add"
          title="Opens another terminal, to run a program at the same time as the others"
          onClick={() => addTerminal()}
        >
          +
        </button>
      </div>
      <div class="terminals" ref={containerRef} />
      <pre class="terminal-text" aria-live="polite">
        {current?.output ?? ''}
      </pre>
      {current && current.state === 'stopped' && (
        <DebugPanel
          key={current.name}
          session={current.session}
          stop={current.stopCount}
          onResume={resume}
          onOpenFile={(path) => editor.open(path)}
        />
      )}
    </section>
  );
}
