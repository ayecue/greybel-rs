import * as vscode from 'vscode';
import { EditorChannel } from 'greybel-api-reference-web/editor';

import { createApiReferencePage } from '../../../src/api-reference/page';
import { PageConnection } from '../../../src/webview/page-connection';
import { extensionUnderTest } from './webview';

export class ApiReferencePage {
  readonly panel: vscode.WebviewPanel;
  #opens = 0;
  #waiting: (() => void) | undefined;

  private constructor() {
    const extension = extensionUnderTest();

    this.panel = vscode.window.createWebviewPanel('greyScriptAPITest', 'API reference test', vscode.ViewColumn.One, {
      enableScripts: true,
      localResourceRoots: [extension.extensionUri]
    });
    new PageConnection(this.panel, (driver) => {
      const channel = new EditorChannel(driver);

      channel.onOpen(() => {
        this.#opens++;
        this.#waiting?.();
      });

      return channel;
    });
    this.panel.webview.html = createApiReferencePage(this.panel.webview, extension.extensionUri);
  }

  static async open(): Promise<ApiReferencePage> {
    const page = new ApiReferencePage();

    await page.opened(1);

    return page;
  }

  opened(count: number): Promise<void> {
    if (this.#opens >= count) return Promise.resolve();

    return new Promise((resolve) => {
      this.#waiting = () => {
        if (this.#opens < count) return;
        resolve();
      };
    });
  }
}
