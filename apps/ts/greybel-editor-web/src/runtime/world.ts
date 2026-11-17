import { WorkerHost } from 'duplex-driver-worker';
import worldScript from 'greyhack-world-operations-web/dist/index.js?url';
import worldWasm from 'greyhack-world-operations-web/dist/greyhack-world-operations-web.wasm?url';

import type { WasmLoader } from '../wasm.js';

/** All terminals share this world until it is reset. */
export class WorldWorker {
  readonly #wasm: WasmLoader;

  #host: Promise<WorkerHost<Record<string, never>>>;
  #seed: string;

  constructor(seed: string, wasm: WasmLoader) {
    this.#wasm = wasm;
    this.#seed = seed;
    this.#host = this.#start(seed);
  }

  get seed(): string {
    return this.#seed;
  }

  async open(): Promise<MessagePort> {
    return (await this.#host).open();
  }

  /** Running programs lose access to the previous world. */
  reset(seed: string): void {
    this.close();
    this.#seed = seed;
    this.#host = this.#start(seed);
  }

  close(): void {
    void this.#host.then((host) => host.close());
  }

  async #start(seed: string): Promise<WorkerHost<Record<string, never>>> {
    const url = new URL(worldScript, location.href);

    url.searchParams.set('seed', seed);

    return new WorkerHost(await this.#wasm.startWorker('world', url, worldWasm), {});
  }
}
