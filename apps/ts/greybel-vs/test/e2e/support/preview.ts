import * as vscode from 'vscode';
import { MessageTargetDriver } from 'duplex-driver-worker';
import { ProgramChannel } from 'greybel-io-web/program';

import { createPreviewPage } from '../../../src/preview/page';
import { WebviewTarget } from '../../../src/webview/target';
import { extensionUnderTest } from './webview';

export interface PreviewPage {
  readonly panel: vscode.WebviewPanel;
  readonly channel: ProgramChannel;
}

export function openPreviewPage(): PreviewPage {
  const extension = extensionUnderTest();
  const panel = vscode.window.createWebviewPanel('greyHackOutputPreviewTest', 'Preview test', vscode.ViewColumn.One, {
    enableScripts: true,
    localResourceRoots: [extension.extensionUri]
  });

  panel.webview.html = createPreviewPage(panel.webview, extension.extensionUri);

  const channel = new ProgramChannel(new MessageTargetDriver(new WebviewTarget(panel.webview)));

  panel.onDidDispose(() => channel.close());

  return { panel, channel };
}
