import type { StopNotice } from 'greybel-runtime';

import { filePath } from '../project/paths.js';
import { describeLocation, RuntimeSession } from './session.js';
import { Terminal } from './terminal.js';
import type { WorldWorker } from './world.js';

export type ProgramState = 'idle' | 'running' | 'stopped';

/** Program stop location within the project. */
export interface StopPlace {
  readonly path: string;
  readonly line: number;
}

/** Program events emitted by a terminal to the page. */
export interface ProgramTerminalListener {
  /** Terminal state, output, or stop location changed. */
  onChange(terminal: ProgramTerminal): void;
  /** Program stopped at a breakpoint, after a step, or after a pause. */
  onStop(terminal: ProgramTerminal): void;
}

/**
 * Canvas-based Grey Hack terminal with its own runtime. Multiple terminals run programs concurrently in the
 * page game server's shared world.
 */
export class ProgramTerminal {
  readonly name: string;
  readonly canvas: HTMLCanvasElement;
  readonly terminal: Terminal;

  state: ProgramState = 'idle';
  /** Current stop location while stopped at a project file. */
  stopPlace: StopPlace | undefined;
  /** Stop counter that prompts the page to reload state after each stop. */
  stopCount = 0;
  /** Terminal text for consumers that cannot read the canvas. */
  output = '';

  #session: RuntimeSession;
  readonly #listener: ProgramTerminalListener;

  /** Creates terminal `name` on a new canvas; throws if canvas drawing is unavailable. */
  constructor(name: string, world: WorldWorker, listener: ProgramTerminalListener) {
    this.name = name;
    this.#listener = listener;
    this.canvas = document.createElement('canvas');
    this.canvas.className = 'terminal';
    this.canvas.title = "Click to type the program's input";
    this.terminal = new Terminal(this.canvas, (text) => {
      this.output = text;
      listener.onChange(this);
    });
    this.#session = this.#open(world);
  }

  get session(): RuntimeSession {
    return this.#session;
  }

  /** Connects to `world` with a new runtime after ending the previous runtime and its program. */
  reconnect(world: WorldWorker): void {
    this.terminal.cancelRead();
    this.#session.close();
    this.#session = this.#open(world);
    this.update('idle');
  }

  /** Sets program state and clears the stop location unless state is `stopped`. */
  update(state: ProgramState): void {
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
    return new RuntimeSession(
      this.terminal,
      {
        onStop: (notice) => this.#stop(notice),
        onLog: (message) => {
          const at = message.location ? `${describeLocation(message.location)}: ` : '';

          this.terminal.printLine(`${at}${message.text}`);
        }
      },
      world.open()
    );
  }

  #stop(notice: StopNotice): void {
    const path = notice.location ? filePath(notice.location.uri) : undefined;

    this.state = 'stopped';
    this.stopPlace = path === undefined || !notice.location ? undefined : { path, line: notice.location.line };
    this.stopCount += 1;
    this.#listener.onStop(this);
    this.#listener.onChange(this);
  }
}
