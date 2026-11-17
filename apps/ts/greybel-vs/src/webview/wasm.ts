/**
 * Compiles WebAssembly from the page script's `data-wasm` URL while the page fetches it; the webview serves
 * only page requests, and the module is used by its worker.
 */
export function loadPageWasm(): Promise<WebAssembly.Module> {
  return WebAssembly.compileStreaming(
    fetch(document.currentScript!.dataset.wasm!).then((response) => {
      if (!response.ok) throw new Error(`the WebAssembly can't be fetched: ${response.status}`);

      return response;
    })
  );
}
