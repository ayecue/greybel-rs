import runtimeWasm from 'greybel-runtime-web/dist/greybel-runtime-web.wasm?url';

let runtimeModule: Promise<WebAssembly.Module> | undefined;

/** greybel-runtime-web WebAssembly module shared by all page runtimes. */
export function compiledRuntime(): Promise<WebAssembly.Module> {
  runtimeModule ??= WebAssembly.compileStreaming(fetch(runtimeWasm)).catch(() =>
    fetch(runtimeWasm)
      .then((response) => response.arrayBuffer())
      .then((bytes) => WebAssembly.compile(bytes))
  );

  return runtimeModule;
}
