import { MessageTargetDriver } from 'duplex-driver-worker';
import type { WebviewPanel } from 'vscode';

import { WebviewTarget } from './target';

export interface Closable {
  close(): void;
}

export class PageConnection {
  readonly #open: (driver: MessageTargetDriver) => Closable;
  readonly #panel: WebviewPanel;
  #page: Closable | undefined;

  constructor(panel: WebviewPanel, open: (driver: MessageTargetDriver) => Closable) {
    this.#panel = panel;
    this.#open = open;
    this.#page = this.#openPage();
    panel.onDidChangeViewState(() => {
      if (panel.visible) this.#page ??= this.#openPage();
      else this.close();
    });
    panel.onDidDispose(() => this.close());
  }

  close(): void {
    this.#page?.close();
    this.#page = undefined;
  }

  #openPage(): Closable {
    return this.#open(new MessageTargetDriver(new WebviewTarget(this.#panel.webview)));
  }
}
