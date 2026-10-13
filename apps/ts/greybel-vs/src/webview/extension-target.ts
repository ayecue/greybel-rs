import type { MessageListener, MessageTarget } from 'duplex-driver-worker';

export interface VsCodeApi {
  postMessage(message: unknown): void;
  getState(): unknown;
  setState(state: unknown): void;
}

declare function acquireVsCodeApi(): VsCodeApi;

let sharedApi: VsCodeApi | undefined;

export function vscodeApi(): VsCodeApi {
  sharedApi ??= acquireVsCodeApi();

  return sharedApi;
}

export class ExtensionTarget implements MessageTarget {
  readonly #vscode = vscodeApi();

  postMessage(message: unknown): void {
    this.#vscode.postMessage(message);
  }

  addEventListener(type: 'message', listener: MessageListener): void {
    window.addEventListener(type, listener);
  }

  removeEventListener(type: 'message', listener: MessageListener): void {
    window.removeEventListener(type, listener);
  }
}
