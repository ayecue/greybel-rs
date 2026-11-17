import { useEffect, useReducer, useRef, useState } from 'preact/hooks';

import type { ProjectEditor } from '../editor/view.js';
import type { Project } from '../project.js';
import { MANIFEST_PATH } from '../project/content.js';
import { DEFAULT_SEED } from '../project/manifest.js';
import type { ProjectSettings } from '../project/settings.js';
import { ProgramTerminal } from '../runtime/program-terminal.js';
import { WorldWorker } from '../runtime/world.js';
import { DebugPanel } from './debug/panel.js';
import { useServices } from './services.js';
import { usePartChanges } from './wasm-status.js';
import { Splitter } from './splitter.js';

const WAIT_TITLE = 'Loading the runtime...';

export interface ExecutePanelConfiguration {
  readonly editor: ProjectEditor;
  readonly project: Project;
  readonly settings: ProjectSettings;
  readonly breakpointChanges: { current: ((path: string, lines: number[]) => void) | undefined };
  readonly onFailure: (message: string) => void;
}

interface TerminalTable {
  readonly world: WorldWorker;
  readonly list: ProgramTerminal[];
  count: number;
}

export function ExecutePanel({ editor, project, settings, breakpointChanges, onFailure }: ExecutePanelConfiguration) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalsRef = useRef<TerminalTable | undefined>(undefined);
  const [current, setCurrent] = useState<ProgramTerminal>();
  const currentRef = useRef<ProgramTerminal | undefined>(undefined);
  const [, refresh] = useReducer((count: number) => count + 1, 0);
  const { wasm } = useServices();
  const isRuntimeReady = wasm.isReady('language server', 'runtime', 'world', 'terminal');

  usePartChanges();

  const showStop = (terminal: ProgramTerminal | undefined) =>
    editor.showStop(terminal?.stopPlace, terminal?.framePlace);

  const selectTerminal = (terminal: ProgramTerminal | undefined) => {
    currentRef.current = terminal;
    setCurrent(terminal);
    for (const other of terminalsRef.current?.list ?? []) other.canvas.hidden = other !== terminal;
    showStop(terminal);
  };

  const addTerminal = (): ProgramTerminal | undefined => {
    const terminals = terminalsRef.current;
    const container = containerRef.current;

    if (terminals === undefined || container === null) return undefined;

    let terminal: ProgramTerminal;

    try {
      terminal = new ProgramTerminal(
        `Terminal ${terminals.count + 1}`,
        terminals.world,
        {
          onChange: (target) => {
            if (target === currentRef.current) showStop(target);
            refresh(undefined);
          },
          onStop: (target) => selectTerminal(target)
        },
        wasm,
        project.paths
      );
    } catch (error) {
      onFailure(`The terminal can't be drawn: ${error instanceof Error ? error.message : String(error)}`);

      return undefined;
    }

    terminals.count += 1;
    terminals.list.push(terminal);
    container.append(terminal.canvas);
    selectTerminal(terminal);

    return terminal;
  };

  const closeTerminal = (terminal: ProgramTerminal) => {
    const list = terminalsRef.current?.list;

    if (list === undefined || list.length === 1) return;

    const index = list.indexOf(terminal);

    list.splice(index, 1);
    terminal.close();
    if (terminal === currentRef.current) selectTerminal(list[Math.min(index, list.length - 1)]);
    else refresh(undefined);
  };

  useEffect(() => {
    terminalsRef.current = { world: new WorldWorker(DEFAULT_SEED, wasm), list: [], count: 0 };
    addTerminal();
    void settings.ready.then(() => {
      const terminals = terminalsRef.current;
      const seed = settings.current?.manifest.run.seed ?? DEFAULT_SEED;

      if (terminals === undefined || terminals.world.seed === seed) return;
      if (terminals.list.some((terminal) => terminal.state !== 'idle')) return;
      terminals.world.reset(seed);
      for (const terminal of terminals.list) terminal.reconnect(terminals.world);
    });
    breakpointChanges.current = (path, lines) => {
      for (const terminal of terminalsRef.current?.list ?? []) {
        void terminal.session.setBreakpoints(project.paths.toUri(path), lines).catch(() => undefined);
      }
    };

    const unsubscribeSettings = settings.subscribe(() => refresh(undefined));
    const unsubscribeProject = project.subscribe((event) => {
      const list = terminalsRef.current?.list ?? [];

      if (event.kind === 'rename') {
        for (const terminal of list) if (terminal.script === event.from) terminal.script = event.to;
      } else if (event.kind === 'files') {
        for (const terminal of list)
          if (terminal.script !== undefined && !project.has(terminal.script)) {
            terminal.script = undefined;
          }
      }
    });

    return () => {
      unsubscribeSettings();
      unsubscribeProject();
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

    terminal.setState('running', debug ? 'debug' : 'run');
    try {
      const script = terminal.script ?? project.entry;
      const runSettings = await settings.read(script, terminal.profile);

      terminal.terminal.setLineLimits(runSettings.lineLimits);

      for (const [uri, lines] of editor.getBreakpoints()) await session.setBreakpoints(uri, lines);

      const summary = session.describeExit(
        await session.run({
          documents: project.documents,
          entry: script,
          settings: runSettings,
          seed: world.seed,
          debug
        })
      );

      if (summary !== undefined) terminal.terminal.printLine(summary);
    } catch (error) {
      if (session === terminal.session) terminal.terminal.printLine(session.describeFailure(error));
    } finally {
      if (session === terminal.session) {
        terminal.terminal.cancelRead();
        terminal.setState('idle');
      }
    }
  };

  const copyTerminal = async () => {
    if (current === undefined) return;
    try {
      await navigator.clipboard.writeText(await current.session.renderPlainText(current.output));
    } catch (error) {
      onFailure(error instanceof Error ? error.message : String(error));
    }
  };

  /** Stops the programs running in the old world. */
  const resetWorld = async () => {
    const terminals = terminalsRef.current;

    if (terminals === undefined) return;

    let seed: string;

    try {
      seed = (await settings.read(MANIFEST_PATH)).manifest.run.seed;
    } catch (error) {
      onFailure(error instanceof Error ? error.message : String(error));

      return;
    }

    terminals.world.reset(seed);
    for (const terminal of terminals.list) {
      if (terminal.state !== 'idle') terminal.terminal.printLine('The program was stopped: the world was reset.');
      terminal.reconnect(terminals.world);
    }

    showStop(currentRef.current);
  };

  const resume = () => {
    current?.setState('running');
  };

  const stop = () => {
    current?.terminal.cancelRead();
    current?.session.terminate();
  };

  const state = current?.state ?? 'idle';
  const isDebugging = current !== undefined && current.debug.isActive && state !== 'idle';
  const list = terminalsRef.current?.list ?? [];
  const profiles = Object.keys(settings.current?.manifest.profiles ?? {});
  const isMissingProfile = current?.profile !== undefined && !profiles.includes(current.profile);

  return (
    <section class="panel execute-panel">
      <header class="panel-header">
        <h2>Execute</h2>
        <button
          type="button"
          class="run"
          disabled={!current || state !== 'idle' || !isRuntimeReady}
          title={isRuntimeReady ? undefined : WAIT_TITLE}
          onClick={() => void run(false)}
        >
          Run
        </button>
        <button
          type="button"
          class="debug"
          disabled={!current || state !== 'idle' || !isRuntimeReady}
          title={isRuntimeReady ? undefined : WAIT_TITLE}
          onClick={() => void run(true)}
        >
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
        {!isDebugging && (
          <button type="button" class="stop" disabled={state === 'idle'} onClick={stop}>
            Stop
          </button>
        )}
        <button type="button" class="clear" onClick={() => current?.terminal.clear()}>
          Clear
        </button>
        <button
          type="button"
          class="copy-terminal"
          disabled={!current || current.output === '' || !wasm.isReady('runtime')}
          title="Copy the terminal's text, without colors and other formatting"
          onClick={() => void copyTerminal()}
        >
          Copy
        </button>
      </header>
      <div class="panel-options">
        <label>
          Script
          <select
            class="script"
            value={current?.script ?? ''}
            disabled={!current}
            title="The file the terminal's runs start at"
            onChange={(event) => {
              if (current === undefined) return;
              current.script = event.currentTarget.value === '' ? undefined : event.currentTarget.value;
              refresh(undefined);
            }}
          >
            <option value="">Entry file ({project.entry})</option>
            {project.scripts.map((path) => (
              <option key={path} value={path}>
                {path}
              </option>
            ))}
          </select>
        </label>
        <label>
          Profile
          <select
            class="profile"
            value={current?.profile ?? ''}
            disabled={!current}
            title="The greybel.json profile whose run settings the terminal's runs use"
            onChange={(event) => {
              if (current === undefined) return;
              current.profile = event.currentTarget.value === '' ? undefined : event.currentTarget.value;
              refresh(undefined);
            }}
          >
            <option value="">Default</option>
            {profiles.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
            {isMissingProfile && <option value={current.profile}>{current.profile} (missing)</option>}
          </select>
        </label>
        <button
          type="button"
          class="reset-world"
          disabled={!isRuntimeReady}
          onClick={() => void resetWorld()}
          title="Generates a new world from run.seed in greybel.json; the programs running in the old one stop"
        >
          New world
        </button>
        <button type="button" class="open-settings" onClick={() => editor.open(MANIFEST_PATH)}>
          greybel.json
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
            onClick={() => selectTerminal(terminal)}
          >
            <span class="terminal-name">{terminal.name}</span>
            <span class="terminal-script">{terminal.script ?? project.entry}</span>
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
      <pre class="terminal-text" aria-live="polite">
        {current?.output ?? ''}
      </pre>
      <div class="terminals" ref={containerRef} />
      {isDebugging && <Splitter name="debug" axis="y" side="after" min={120} />}
      {isDebugging && <DebugPanel key={current.name} terminal={current} onResume={resume} onStop={stop} />}
    </section>
  );
}
