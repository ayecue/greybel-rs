import { WorkerHost, type WorkerHostOptions } from 'duplex-driver-worker';
import { ExtensionContext, Uri } from 'vscode';
import { serveWasm } from 'wasm-host/worker';

import type { Launcher, Peer } from '../launcher';

export class WorkerLauncher implements Launcher {
  readonly #context: ExtensionContext;
  readonly #name: string;
  readonly #options: WorkerHostOptions;

  constructor(context: ExtensionContext, name: string, options: WorkerHostOptions = {}) {
    this.#context = context;
    this.#name = name;
    this.#options = options;
  }

  /** Starts dist/`name`.js with `parameters` in its URL query and supplies `name`.wasm. */
  start(parameters: Record<string, string> = {}): Worker {
    const query = new URLSearchParams(parameters).toString();
    const worker = new Worker(`${this.#getDistUrl(`${this.#name}.js`)}${query === '' ? '' : `?${query}`}`);

    serveWasm(worker, this.#getDistUrl(`${this.#name}.wasm`));

    return worker;
  }

  launch(): Peer {
    const host = new WorkerHost(this.start(), {}, this.#options);

    return {
      transport: host.transport,
      stop: () => host.close()
    };
  }

  /** Returns the URL for extension file `file` under dist/. */
  #getDistUrl(file: string): string {
    return Uri.joinPath(this.#context.extensionUri, 'dist', file).toString(true);
  }
}
