import * as vscode from 'vscode';
import { TerminalChannel } from 'greyhack-terminal-text';

import type { Peer } from '../../../src/launcher';
import { ProcessLauncher } from '../../../src/launcher/process';
import { WorkerLauncher } from '../../../src/launcher/worker';
import { TerminalPage } from '../../../src/runtime/terminal-page';
import { extensionUnderTest } from './webview';

export class RecordedTerminal {
  readonly page: TerminalPage;
  readonly written: string[] = [];
  readonly #peer: Peer;

  constructor() {
    const context = { extensionUri: extensionUnderTest().extensionUri } as vscode.ExtensionContext;

    this.#peer =
      vscode.env.uiKind === vscode.UIKind.Web
        ? new WorkerLauncher(context, 'runtime-web').launch()
        : new ProcessLauncher(context, 'greybel-runtime').launch();

    this.page = new TerminalPage(new TerminalChannel(this.#peer.transport), { show: () => undefined });
    this.page.terminal.outputEmitter.event((data) => this.written.push(data));
  }

  async sent(data: string): Promise<void> {
    while (!this.written.join('').includes(data)) await new Promise((resolve) => setTimeout(resolve, 50));
  }

  dispose(): void {
    this.page.terminal.dispose();
    this.#peer.stop();
  }
}
