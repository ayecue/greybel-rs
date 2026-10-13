import * as assert from 'assert';
import * as vscode from 'vscode';
import { ChannelError, ErrorCode } from 'duplex-core';
import { Echo, ReadLine } from 'greybel-io-web/program';

import { openPreviewPage } from './support/preview';
import { webviewTabNamed } from './support/webview';
import { TIMEOUT } from './support/workspace';

describe('output preview', function () {
  this.timeout(TIMEOUT);

  it('should open the preview as a webview', async () => {
    const command = 'greybel.preview';

    await vscode.commands.executeCommand(command);

    const tab = await webviewTabNamed('Grey Hack Output Preview');

    assert.ok(tab.input instanceof vscode.TabInputWebview);
  });

  it('should respond to the program over the webview once the page draws', async () => {
    const { panel, channel } = openPreviewPage();
    const response = channel.send(new ReadLine({ prompt: 'name: ', echo: Echo.Visible }));

    try {
      await assert.rejects(
        response,
        (error: unknown) => error instanceof ChannelError && error.code === ErrorCode.UnhandledMessage
      );
    } finally {
      panel.dispose();
    }
  });
});
