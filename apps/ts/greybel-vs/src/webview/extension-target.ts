import type { MessageListener, MessageTarget } from 'duplex-driver-worker';

export interface VsCodeApi {
  postMessage(message: unknown): void;
  getState(): unknown;
  setState(state: unknown): void;
}

declare function acquireVsCodeApi(): VsCodeApi;

export class ExtensionApi implements VsCodeApi {
  readonly #api = acquireVsCodeApi();

  postMessage(message: unknown): void {
    this.#api.postMessage(message);
  }

  getState(): unknown {
    return this.#api.getState();
  }

  setState(state: unknown): void {
    this.#api.setState(state);
  }
}

export class ExtensionTarget implements MessageTarget {
  readonly #vscode: VsCodeApi;

  constructor(vscode: VsCodeApi) {
    this.#vscode = vscode;
  }

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
