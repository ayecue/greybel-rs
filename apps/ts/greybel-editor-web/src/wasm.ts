import { serveWasm } from 'wasm-host/worker';

export type WasmPart = 'language server' | 'transformer' | 'runtime' | 'world' | 'terminal' | 'API reference';

export interface WasmFailure {
  readonly part: WasmPart;
  readonly message: string;
}

/** Compiles each WebAssembly module once and tracks which parts of the page are loading, ready or failed. */
export class WasmLoader {
  readonly #modules = new Map<string, Promise<WebAssembly.Module>>();
  readonly #unsettledCounts = new Map<WasmPart, number>();
  readonly #readyParts = new Set<WasmPart>();
  readonly #failures = new Map<WasmPart, string>();
  readonly #listeners = new Set<() => void>();

  get incompleteParts(): WasmPart[] {
    return [...this.#unsettledCounts.keys()].filter((part) => !this.#failures.has(part));
  }

  get failures(): WasmFailure[] {
    return [...this.#failures].map(([part, message]) => ({ part, message }));
  }

  /**
   * Starts a worker running `scriptUrl` once `part`'s module is compiled, and passes it the module. A channel to a
   * worker still downloading its module fails its health check after about 20 s, so workers start only when they
   * can respond.
   */
  async startWorker(part: WasmPart, scriptUrl: string | URL, wasmUrl: string): Promise<Worker> {
    const module = await this.load(part, wasmUrl);
    const worker = new Worker(scriptUrl);

    serveWasm(worker, module);

    return worker;
  }

  /** On failure the part is shown as failed and the promise never settles, so workers wait instead of throwing. */
  load(part: WasmPart, url: string): Promise<WebAssembly.Module> {
    const module = this.#compile(url);

    this.track(part, module);

    return module.catch(() => new Promise<never>(() => undefined));
  }

  /** A part is ready once all its tracked promises resolved. */
  track(part: WasmPart, promise: Promise<unknown>): void {
    this.#unsettledCounts.set(part, (this.#unsettledCounts.get(part) ?? 0) + 1);
    this.#notifyListeners();
    promise.then(
      () => this.#settle(part),
      (error: unknown) => {
        this.#failures.set(part, error instanceof Error ? error.message : String(error));
        this.#settle(part);
      }
    );
  }

  isReady(...parts: readonly WasmPart[]): boolean {
    return parts.every((part) => this.#readyParts.has(part));
  }

  /** The incomplete parts as a list in a sentence, such as `language server, runtime and world`. */
  describeIncompleteParts(): string {
    const parts = this.incompleteParts;

    if (parts.length < 2) return parts.join('');

    return `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)}`;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);

    return () => this.#listeners.delete(listener);
  }

  /** Compiles from the bytes when the response is not served as `application/wasm`. */
  #compile(url: string): Promise<WebAssembly.Module> {
    let module = this.#modules.get(url);

    if (module === undefined) {
      module = WebAssembly.compileStreaming(fetch(url)).catch(async () => {
        const response = await fetch(url);

        if (!response.ok) throw new Error(`the server responded ${response.status} ${response.statusText}`.trim());

        return WebAssembly.compile(await response.arrayBuffer());
      });
      this.#modules.set(url, module);
    }

    return module;
  }

  #settle(part: WasmPart): void {
    const count = (this.#unsettledCounts.get(part) ?? 1) - 1;

    if (count > 0) {
      this.#unsettledCounts.set(part, count);
    } else {
      this.#unsettledCounts.delete(part);
      if (!this.#failures.has(part)) this.#readyParts.add(part);
    }

    this.#notifyListeners();
  }

  #notifyListeners(): void {
    for (const listener of this.#listeners) listener();
  }
}
