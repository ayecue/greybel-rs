import { execFile, spawn, type ChildProcess } from 'child_process';
import { createInterface } from 'readline';

import type { WorldHost } from '../launcher';

/** Settings for the world server shared by this machine's windows and Greybel commands. */
export interface SharedWorldSettings {
  /** 0 makes the window run its own world server. */
  readonly port: number;
  /** Seconds without connections after which a shared server this window starts stops; 0 never stops it. */
  readonly idleTimeout: number;
  /** Folder in which started servers save the world of each seed; empty for `greybel/worlds` in the data folder. */
  readonly folder: string;
}

/** World server process this window started for itself, and its listening address. */
interface OwnServer {
  readonly child: ChildProcess;
  readonly seed: string;
  readonly address: Promise<string>;
}

/** Result of `greybel-runtime world locate`: the shared server's address, or why it can't be used. */
type Location = { readonly address: string } | { readonly reason: string };

/**
 * Provides world servers for the window's debug sessions. Uses this machine's shared server when usable for the seed;
 * otherwise starts a window-specific greyhack-world-server.
 */
export class WorldProcess implements WorldHost<string> {
  readonly #server: string;
  readonly #runtime: string;
  readonly #settings: () => SharedWorldSettings;
  readonly #report: (reason: string) => void;
  readonly #reported = new Set<string>();
  #own: OwnServer | undefined;
  /** Port of the last shared server used, for reset. */
  #sharedPort: number | undefined;

  /**
   * Uses `server` to start servers and `runtime` to locate the shared server. `report` receives each reason the shared
   * server is unusable once.
   */
  constructor(server: string, runtime: string, settings: () => SharedWorldSettings, report: (reason: string) => void) {
    this.#server = server;
    this.#runtime = runtime;
    this.#settings = settings;
    this.#report = report;
  }

  /** Returns the address of a server running `seed`, starting one if needed. */
  async connect(seed: string): Promise<string> {
    if (this.#own?.seed === seed && this.#own.child.exitCode === null) return this.#own.address;

    const { port, idleTimeout, folder } = this.#settings();
    const worlds = folder === '' ? [] : ['--worlds', folder];

    if (port !== 0) {
      const location = await this.#locate(port, seed, idleTimeout, worlds);

      if ('address' in location) {
        this.#stopOwn();
        this.#sharedPort = port;

        return location.address;
      }

      if (!this.#reported.has(location.reason)) {
        this.#reported.add(location.reason);
        this.#report(location.reason);
      }
    }

    return this.#startOwn(seed, worlds);
  }

  /** Regenerates the world, interrupting programs running in it. */
  async reset(): Promise<void> {
    this.#stopOwn();
    if (this.#sharedPort === undefined) return;

    await this.#runWorld(['reset', '--port', String(this.#sharedPort)]);
  }

  /** Stops only the window's own server; the shared server continues for others. */
  dispose(): void {
    this.#stopOwn();
  }

  async #locate(port: number, seed: string, idleTimeout: number, worlds: string[]): Promise<Location> {
    const result = await this.#runWorld([
      'locate',
      '--port',
      String(port),
      '--seed',
      seed,
      '--idle-timeout',
      String(idleTimeout),
      '--server',
      this.#server,
      ...worlds
    ]);

    return result.code === 0 ? { address: result.stdout.trim() } : { reason: result.stderr.trim() };
  }

  #runWorld(args: string[]): Promise<{ code: number; stdout: string; stderr: string }> {
    return new Promise((resolve) => {
      execFile(this.#runtime, ['world', ...args], (error, stdout, stderr) => {
        const failure = typeof error?.code === 'number' ? error.code : 1;
        const code = error === null ? 0 : failure;

        resolve({ code, stdout, stderr: stderr || (error?.message ?? '') });
      });
    });
  }

  #startOwn(seed: string, worlds: string[]): Promise<string> {
    this.#stopOwn();

    const child = spawn(this.#server, ['--seed', seed, ...worlds], { stdio: ['pipe', 'pipe', 'inherit'] });
    const address = new Promise<string>((resolve, reject) => {
      const lines = createInterface({ input: child.stdout! });

      lines.once('line', (port) => {
        lines.close();
        resolve(`127.0.0.1:${port.trim()}`);
      });
      child.once('error', reject);
      child.once('exit', (code) => reject(new Error(`the world server ended (${code})`)));
    });

    this.#own = { child, seed, address };

    return address;
  }

  #stopOwn(): void {
    this.#own?.child.kill();
    this.#own = undefined;
  }
}
