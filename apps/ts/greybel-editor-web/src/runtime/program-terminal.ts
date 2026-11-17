import type { StopNotice } from 'greybel-runtime';

import type { PathResolver } from '../project/paths.js';
import type { WasmLoader } from '../wasm.js';
import { RuntimeSession } from './session.js';
import { Terminal } from './terminal.js';
import type { WorldWorker } from './world.js';

export type ProgramState = 'idle' | 'running' | 'stopped';

export interface StopPlace {
  readonly path: string;
  readonly line: number;
}

export interface ProgramTerminalListener {
  onChange(terminal: ProgramTerminal): void;
  onStop(terminal: ProgramTerminal): void;
}

export class ProgramTerminal {
  readonly name: string;
  readonly canvas: HTMLCanvasElement;
  readonly terminal: Terminal;

  /** Undefined runs the project's entry file. */
  script: string | undefined;
  /** Undefined uses the top-level settings. */
  profile: string | undefined;

  state: ProgramState = 'idle';
  stopPlace: StopPlace | undefined;
  /** Incremented on each stop. */
  stopCount = 0;
  /** Plain text for screen readers. */
  output = '';

  #session: RuntimeSession;
  readonly #listener: ProgramTerminalListener;
  readonly #wasm: WasmLoader;
  readonly #paths: PathResolver;

  /** Throws when the canvas cannot be drawn. */
  constructor(
    name: string,
    world: WorldWorker,
    listener: ProgramTerminalListener,
    wasm: WasmLoader,
    paths: PathResolver
  ) {
    this.name = name;
    this.#listener = listener;
    this.#wasm = wasm;
    this.#paths = paths;
    this.canvas = document.createElement('canvas');
    this.canvas.className = 'terminal';
    this.canvas.title = "Click to type the program's input";
    this.terminal = new Terminal(
      this.canvas,
      (text) => {
        this.output = text;
        listener.onChange(this);
      },
      wasm
    );
    this.#session = this.#open(world);
  }

  get session(): RuntimeSession {
    return this.#session;
  }

  reconnect(world: WorldWorker): void {
    this.terminal.cancelRead();
    this.#session.close();
    this.#session = this.#open(world);
    this.setState('idle');
  }

  /** Clears the stop location unless `state` is `stopped`. */
  setState(state: ProgramState): void {
    this.state = state;
    if (state !== 'stopped') this.stopPlace = undefined;
    this.#listener.onChange(this);
  }

  close(): void {
    this.#session.close();
    this.terminal.close();
    this.canvas.remove();
  }

  #open(world: WorldWorker): RuntimeSession {
    const session: RuntimeSession = new RuntimeSession(
      this.terminal,
      {
        onStop: (notice) => this.#stop(notice),
        onLog: (message) => {
          const at = message.location ? `${session.describeLocation(message.location)}: ` : '';

          this.terminal.printLine(`${at}${message.text}`);
        }
      },
      world.open(),
      this.#wasm,
      this.#paths
    );

    return session;
  }

  #stop(notice: StopNotice): void {
    const path = notice.location ? this.#paths.toPath(notice.location.uri) : undefined;

    this.state = 'stopped';
    this.stopPlace = path === undefined || !notice.location ? undefined : { path, line: notice.location.line };
    this.stopCount += 1;
    this.#listener.onStop(this);
    this.#listener.onChange(this);
  }
}
