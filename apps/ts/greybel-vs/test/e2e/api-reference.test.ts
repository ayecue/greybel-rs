import * as assert from 'assert';
import * as vscode from 'vscode';
import { OpenDocument } from 'greybel-api-reference-web/editor';

import { WorkbenchEditor } from '../../src/api-reference/workbench-editor';
import { ApiReferencePage } from './support/api-reference';
import { webviewTabNamed } from './support/webview';
import { TIMEOUT } from './support/workspace';

describe('API reference', function () {
  this.timeout(TIMEOUT);

  it('should open the reference as a webview', async () => {
    await vscode.commands.executeCommand('greybel.api');

    const tab = await webviewTabNamed('GreyScript API');

    assert.ok(tab.input instanceof vscode.TabInputWebview);
  });

  it("should connect the reference's page to the editor", async () => {
    const page = await ApiReferencePage.open();

    page.panel.dispose();
  });

  it('should connect the page the webview loads again after it was hidden', async () => {
    const page = await ApiReferencePage.open();
    const document = await vscode.workspace.openTextDocument({ content: '', language: 'greyscript' });

    await vscode.window.showTextDocument(document, vscode.ViewColumn.One);
    page.panel.reveal(vscode.ViewColumn.One);
    await page.opened(2);
    page.panel.dispose();
    await vscode.window.showTextDocument(document, vscode.ViewColumn.One);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
  });

  it('should open an example as an untitled GreyScript document', async () => {
    await new WorkbenchEditor().onOpenDocument(new OpenDocument({ content: 'print(get_shell)' }));

    const document = vscode.window.activeTextEditor?.document;
    const opened = { isUntitled: document?.isUntitled, languageId: document?.languageId, text: document?.getText() };

    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    assert.deepStrictEqual(opened, { isUntitled: true, languageId: 'greyscript', text: 'print(get_shell)' });
  });
});
