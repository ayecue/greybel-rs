/** Worker message key used by the host to supply crate WebAssembly: `{ [WASM_KEY]: source }`. */
export const WASM_KEY = 'wasm-host:wasm';

export type WasmSource = string | URL | BufferSource | WebAssembly.Module;

export interface WasmTarget {
  postMessage(message: unknown): void;
}

export function serveWasm(worker: WasmTarget, source: WasmSource | PromiseLike<WasmSource>): void {
  void Promise.resolve(source).then((wasm) =>
    worker.postMessage({ [WASM_KEY]: wasm instanceof URL ? wasm.href : wasm })
  );
}
