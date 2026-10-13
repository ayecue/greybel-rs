import { Transport } from 'duplex-core';
import { MessageTargetDriver, type MessageTarget } from 'duplex-driver-worker';
import { serveWasm, type WasmSource } from 'wasm-host/worker';

import { TerminalPage } from './terminal-page.js';
import TerminalWorker from './worker.ts?worker&inline';

/**
 * Draws the program terminal from `target` onto `canvas`. `wasm` accepts the package's
 * `greyhack-terminal-web.wasm` URL, bytes, module, or a promise for one.
 */
export function openTerminal(
  canvas: HTMLCanvasElement,
  target: MessageTarget,
  wasm: WasmSource | PromiseLike<WasmSource>
): TerminalPage {
  const worker = new TerminalWorker();

  serveWasm(worker, wasm);

  return new TerminalPage(canvas, worker, new Transport(new MessageTargetDriver(target)));
}
