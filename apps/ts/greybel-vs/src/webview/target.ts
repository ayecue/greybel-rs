import type { MessageListener, MessageTarget } from 'duplex-driver-worker';
import type { Disposable, Webview } from 'vscode';

export class WebviewTarget implements MessageTarget {
  readonly #webview: Webview;
  readonly #listeners = new Map<MessageListener, Disposable>();

  constructor(webview: Webview) {
    this.#webview = webview;
  }

  postMessage(message: unknown): void {
    void this.#webview.postMessage(message);
  }

  addEventListener(_type: 'message', listener: MessageListener): void {
    if (this.#listeners.has(listener)) return;
    this.#listeners.set(
      listener,
      this.#webview.onDidReceiveMessage((data: unknown) => listener({ data }))
    );
  }

  removeEventListener(_type: 'message', listener: MessageListener): void {
    this.#listeners.get(listener)?.dispose();
    this.#listeners.delete(listener);
  }
}
