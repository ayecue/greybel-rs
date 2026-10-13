import vscode, { ExtensionContext } from 'vscode';
import { MessageTargetDriver } from 'duplex-driver-worker';
import { Output } from 'greybel-io-web';
import { OutputBatch, ProgramChannel } from 'greybel-io-web/program';

import { previewPage } from './preview/page';
import { WebviewTarget } from './webview/target';

class Preview {
  private context: ExtensionContext;
  private panel: vscode.WebviewPanel | null;
  private channel: ProgramChannel | null;

  constructor(context: ExtensionContext) {
    this.context = context;
    this.panel = null;
    this.channel = null;
  }

  open() {
    if (this.panel) {
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      'greyHackOutputPreview',
      'Grey Hack Output Preview',
      {
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true
      },
      {
        enableScripts: true,
        localResourceRoots: [this.context.extensionUri],
        retainContextWhenHidden: true
      }
    );

    panel.webview.html = previewPage(panel.webview, this.context.extensionUri);

    const channel = new ProgramChannel(new MessageTargetDriver(new WebviewTarget(panel.webview)));

    this.panel = panel;
    this.channel = channel;

    panel.onDidDispose(() => {
      channel.close();
      this.panel = null;
      this.channel = null;
    });
  }

  print(
    message: string,
    { appendNewLine = true, replace = false }: { appendNewLine?: boolean; replace?: boolean } = {}
  ) {
    this.send(new Output.Print({ text: message, newLine: appendNewLine, replace }));
  }

  clear() {
    this.send(new Output.Clear());
  }

  printInline(message: string) {
    this.send(new Output.Print({ text: message, newLine: false, replace: false }));
  }

  replaceLast(message: string) {
    this.send(new Output.Print({ text: message, newLine: false, replace: true }));
  }

  show(entries: Output[]) {
    this.channel?.send(new OutputBatch({ entries }));
  }

  showPrompt(message: string) {
    this.printInline(message);
  }

  private send(output: Output) {
    this.channel?.send(new OutputBatch({ entries: [output] }));
  }
}

let preview: Preview = null;

export function getPreview() {
  return preview;
}

export function setPreview(instance: Preview) {
  preview = instance;
}

export function activate(context: ExtensionContext) {
  setPreview(new Preview(context));

  context.subscriptions.push(vscode.commands.registerCommand('greybel.preview', () => getPreview().open()));
}
