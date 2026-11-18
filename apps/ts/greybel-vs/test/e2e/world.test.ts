import * as assert from 'assert';
import * as vscode from 'vscode';

import { WRITE, readContent, run } from './support/world';
import { TIMEOUT } from './support/workspace';

/** Shared world server of every test: apart from the default port that a developer's editors use, and stopped soon. */
const WORLD_SETTINGS = { 'world.port': 41799, 'world.idleTimeout': 5 };

// Root hooks, which apply to the tests of every file.
before(async () => {
  for (const [key, value] of Object.entries(WORLD_SETTINGS)) {
    await vscode.workspace.getConfiguration('greybel').update(key, value, vscode.ConfigurationTarget.Workspace);
  }
});

after(async () => {
  for (const key of Object.keys(WORLD_SETTINGS)) {
    await vscode.workspace.getConfiguration('greybel').update(key, undefined, vscode.ConfigurationTarget.Workspace);
  }
});

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
