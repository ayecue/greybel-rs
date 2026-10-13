import * as vscode from 'vscode';
import { TerminalChannel } from 'greyhack-terminal-text';

import type { Peer } from '../../../src/launcher';
import { createProcessLauncher } from '../../../src/launcher/process';
import { createWorkerLauncher } from '../../../src/launcher/worker';
import { setPreview } from '../../../src/preview';
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
        ? createWorkerLauncher(context, 'runtime-web')()
        : createProcessLauncher(context, 'greybel-runtime')();

    setPreview({ show: () => undefined } as unknown as Parameters<typeof setPreview>[0]);
    this.page = new TerminalPage(new TerminalChannel(this.#peer.transport));
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
