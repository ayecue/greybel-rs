import * as assert from 'assert';
import * as vscode from 'vscode';

import { editorWith, ENTRY, OUTPUT, waitForFiles } from './support/build';
import { removeManifest } from './support/project';
import { TIMEOUT, fixture } from './support/workspace';

describe('building and transforming in the builder', function () {
  this.timeout(TIMEOUT);

  before(async () => {
    const config = vscode.workspace.getConfiguration('greybel');

    await config.update('transpiler.buildType', 'Default', vscode.ConfigurationTarget.Workspace);
    await config.update('transpiler.installer.active', true, vscode.ConfigurationTarget.Workspace);
  });

  after(async () => {
    const config = vscode.workspace.getConfiguration('greybel');

    await config.update('transpiler.buildType', undefined, vscode.ConfigurationTarget.Workspace);
    await config.update('transpiler.installer.active', undefined, vscode.ConfigurationTarget.Workspace);
    await removeManifest();
    await vscode.workspace.fs.delete(fixture(OUTPUT), { recursive: true }).then(undefined, () => undefined);
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
  });

  it("should beautify the editor's code", async () => {
    const editor = await editorWith('a=1\nif a==1 then print("one")');
    const changed = new Promise<void>((resolve) => {
      const listener = vscode.workspace.onDidChangeTextDocument(({ document }) => {
        if (document !== editor.document) return;
        listener.dispose();
        resolve();
      });
    });

    await vscode.commands.executeCommand('greybel.beautify.write');
    await changed;

    assert.strictEqual(editor.document.getText(), 'a = 1\nif a == 1 then print("one")');
  });

  it('should build a file with the files it imports, and its installer', async () => {
    await vscode.commands.executeCommand('greybel.buildFileFromContext', fixture(ENTRY));

    const output = fixture(`${OUTPUT}`);
    const entries = await waitForFiles(output, 3);
    const main = new TextDecoder().decode(await vscode.workspace.fs.readFile(vscode.Uri.joinPath(output, 'entry.src')));

    assert.deepStrictEqual(entries.sort(), ['entry.src', 'installer0.src', 'lib.src']);

    assert.ok(/import_code\("\/root\/[^"]*\/lib\.src"\)/.test(main), main);
  });
});
