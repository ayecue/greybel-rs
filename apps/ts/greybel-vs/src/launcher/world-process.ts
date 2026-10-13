import { spawn, type ChildProcess } from 'child_process';
import { createInterface } from 'readline';

import type { WorldHost } from '../launcher';

/** Running game server process and its listening address. */
interface GameServerProcess {
  readonly child: ChildProcess;
  readonly seed: string;
  readonly address: Promise<string>;
}

/**
 * Window greyhack-world-server process started on demand. All debug-session runtimes connect to it and share
 * its world.
 */
export class WorldProcess implements WorldHost<string> {
  readonly #program: string;
  #current: GameServerProcess | undefined;

  constructor(program: string) {
    this.#program = program;
  }

  /** Starts the game server for `seed` if needed and returns its address. */
  connect(seed: string): Promise<string> {
    if (this.#current?.seed !== seed || this.#current.child.exitCode !== null) this.#start(seed);

    return this.#current!.address;
  }

  /** Stops the game server so the next program uses a new world. */
  reset(): void {
    this.#current?.child.kill();
    this.#current = undefined;
  }

  #start(seed: string): void {
    this.reset();

    const child = spawn(this.#program, ['--seed', seed], { stdio: ['pipe', 'pipe', 'inherit'] });
    const address = new Promise<string>((resolve, reject) => {
      const lines = createInterface({ input: child.stdout! });

      lines.once('line', (port) => {
        lines.close();
        resolve(`127.0.0.1:${port.trim()}`);
      });
      child.once('error', reject);
      child.once('exit', (code) => reject(new Error(`the game server ended (${code})`)));
    });

    this.#current = { child, seed, address };
  }
}
