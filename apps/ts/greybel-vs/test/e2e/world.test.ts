import * as assert from 'assert';
import * as vscode from 'vscode';

import { WRITE, readContent, run } from './support/world';
import { TIMEOUT } from './support/workspace';

describe('the mock world the debug sessions share', function () {
  this.timeout(TIMEOUT);

  before(async () => {
    await vscode.workspace
      .getConfiguration('greybel')
      .update('interpreter.useDefaultArgs', true, vscode.ConfigurationTarget.Workspace);
    await vscode.commands.executeCommand('greybel.resetWorld');
  });

  after(async () => {
    await vscode.workspace
      .getConfiguration('greybel')
      .update('interpreter.useDefaultArgs', undefined, vscode.ConfigurationTarget.Workspace);
  });

  it("should let a session's program read what another session's program wrote", async () => {
    await run(WRITE);

    assert.strictEqual(await readContent(), '"from the writer"');
  });

  it('should start a new world once it is reset', async () => {
    await run(WRITE);
    await vscode.commands.executeCommand('greybel.resetWorld');

    assert.strictEqual(await readContent(), 'null');
  });
});
