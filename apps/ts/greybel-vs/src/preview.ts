import vscode, { ExtensionContext } from 'vscode';
import { MessageTargetDriver } from 'duplex-driver-worker';
import { Output } from 'greybel-io-web';
import { OutputBatch, ProgramChannel } from 'greybel-io-web/program';

import { createPreviewPage } from './preview/page';
import type { TerminalLineLimits } from './project';
import { WebviewTarget } from './webview/target';

export interface PreviewOutput {
  show(entries: Output[]): void;
}

export class Preview implements PreviewOutput {
  readonly #context: ExtensionContext;
  #panel: vscode.WebviewPanel | null = null;
  #channel: ProgramChannel | null = null;
  #lineLimits: TerminalLineLimits | undefined;

  constructor(context: ExtensionContext) {
    this.#context = context;
    context.subscriptions.push(vscode.commands.registerCommand('greybel.preview', () => this.open()));
  }

  open(): void {
    if (this.#panel) return;

    const panel = vscode.window.createWebviewPanel(
      'greyHackOutputPreview',
      'Grey Hack Output Preview',
      {
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true
      },
      {
        enableScripts: true,
        localResourceRoots: [this.#context.extensionUri],
        retainContextWhenHidden: true
      }
    );

    panel.webview.html = createPreviewPage(panel.webview, this.#context.extensionUri);

    const channel = new ProgramChannel(new MessageTargetDriver(new WebviewTarget(panel.webview)));

    this.#panel = panel;
    this.#channel = channel;
    this.#postLineLimits();

    panel.onDidDispose(() => {
      channel.close();
      this.#panel = null;
      this.#channel = null;
    });
  }

  /** `undefined` splits long lines as the game does. */
  setLineLimits(limits: TerminalLineLimits | undefined): void {
    this.#lineLimits = limits;
    this.#postLineLimits();
  }

  print(
    message: string,
    { appendNewLine = true, replace = false }: { appendNewLine?: boolean; replace?: boolean } = {}
  ): void {
    this.#send(new Output.Print({ text: message, newLine: appendNewLine, replace }));
  }

  clear(): void {
    this.#send(new Output.Clear());
  }

  printInline(message: string): void {
    this.#send(new Output.Print({ text: message, newLine: false, replace: false }));
  }

  replaceLast(message: string): void {
    this.#send(new Output.Print({ text: message, newLine: false, replace: true }));
  }

  show(entries: Output[]): void {
    this.#channel?.send(new OutputBatch({ entries }));
  }

  showPrompt(message: string): void {
    this.printInline(message);
  }

  #postLineLimits(): void {
    void this.#panel?.webview.postMessage({ lineLimits: this.#lineLimits ?? null });
  }

  #send(output: Output): void {
    this.#channel?.send(new OutputBatch({ entries: [output] }));
  }
}
