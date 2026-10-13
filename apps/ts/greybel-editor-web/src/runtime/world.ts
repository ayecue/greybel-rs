import { WorkerHost } from 'duplex-driver-worker';
import worldScript from 'greyhack-world-operations-web/dist/index.js?url';
import worldWasm from 'greyhack-world-operations-web/dist/greyhack-world-operations-web.wasm?url';
import { serveWasm } from 'wasm-host/worker';

/**
 * Runs the page's greyhack-world-operations-web game server in a worker. All terminal programs share its
 * world and observe each other's changes until reset.
 */
export class WorldWorker {
  #host: WorkerHost<Record<string, never>>;
  #seed: string;

  constructor(seed: string) {
    this.#seed = seed;
    this.#host = WorldWorker.#start(seed);
  }

  /** World-generation seed. */
  get seed(): string {
    return this.#seed;
  }

  /** New world-connection port for a runtime world channel. */
  open(): MessagePort {
    return this.#host.open();
  }

  /** Replaces the world using `seed`; running programs lose access to the previous world. */
  reset(seed: string): void {
    this.#host.close();
    this.#seed = seed;
    this.#host = WorldWorker.#start(seed);
  }

  close(): void {
    this.#host.close();
  }

  static #start(seed: string): WorkerHost<Record<string, never>> {
    const url = new URL(worldScript, location.href);

    url.searchParams.set('seed', seed);

    const worker = new Worker(url);

    serveWasm(worker, worldWasm);

    return new WorkerHost(worker, {});
  }
}
