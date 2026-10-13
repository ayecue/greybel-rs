import vscode, { ExtensionContext } from 'vscode';
import { EditorChannel } from 'greybel-api-reference-web/editor';

import { apiReferencePage } from './api-reference/page';
import { WorkbenchEditor } from './api-reference/workbench-editor';
import { PageConnection } from './webview/page-connection';

class ApiReference {
  readonly #context: ExtensionContext;
  #panel: vscode.WebviewPanel | undefined;

  constructor(context: ExtensionContext) {
    this.#context = context;
  }

  open(): void {
    if (this.#panel !== undefined) {
      this.#panel.reveal();

      return;
    }

    const panel = vscode.window.createWebviewPanel('greyScriptAPI', 'GreyScript API', vscode.ViewColumn.Beside, {
      enableScripts: true,
      localResourceRoots: [this.#context.extensionUri]
    });

    new PageConnection(panel, (driver) => new EditorChannel(driver).use(new WorkbenchEditor()));
    panel.webview.html = apiReferencePage(panel.webview, this.#context.extensionUri);
    panel.onDidDispose(() => {
      this.#panel = undefined;
    });
    this.#panel = panel;
  }
}

export function activate(context: ExtensionContext) {
  const reference = new ApiReference(context);

  context.subscriptions.push(vscode.commands.registerCommand('greybel.api', () => reference.open()));
}
