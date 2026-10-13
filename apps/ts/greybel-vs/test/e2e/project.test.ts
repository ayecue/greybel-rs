import * as assert from 'assert';
import * as vscode from 'vscode';

import { ENTRY, waitForFiles } from './support/build';
import { obfuscation, readManifest, removeManifest, until, writeManifest } from './support/project';
import { TIMEOUT, fixture } from './support/workspace';

describe("the project's greybel.json", function () {
  this.timeout(TIMEOUT);

  const settings = () => vscode.workspace.getConfiguration('greybel', fixture(ENTRY));

  before(removeManifest);

  afterEach(async () => {
    await removeManifest();
    await until(() =>
      ['transpiler.obfuscation', 'interpreter.seed', 'autoCompile', 'transpiler.installer.active'].every(
        (key) => settings().inspect(key)?.workspaceValue === undefined
      )
    );
    await vscode.workspace.fs.delete(fixture('dist'), { recursive: true }).then(undefined, () => undefined);
  });

  it("should get the workspace's settings, and lose them with them", async () => {
    await settings().update('transpiler.obfuscation', true, vscode.ConfigurationTarget.Workspace);

    await until(async () => obfuscation(await readManifest()) === true);

    await settings().update('transpiler.obfuscation', undefined, vscode.ConfigurationTarget.Workspace);

    await until(async () => obfuscation(await readManifest()) === undefined);
  });

  it('should give its settings to the workspace, and take them away with them', async () => {
    await writeManifest({ run: { seed: 'synced' } });

    await until(() => settings().inspect('interpreter.seed')?.workspaceValue === 'synced');
    assert.deepStrictEqual(await readManifest(), { run: { seed: 'synced' } });

    await writeManifest({});

    await until(() => settings().inspect('interpreter.seed')?.workspaceValue === undefined);
  });

  it('should build as it says', async () => {
    await writeManifest({ build: { output: 'dist', autoCompile: false, installer: { active: false } } });
    await until(() => settings().inspect('transpiler.installer.active')?.workspaceValue === false);

    await vscode.commands.executeCommand('greybel.buildFileFromContext', fixture(ENTRY));

    const entries = await waitForFiles(fixture('dist'), 2);

    assert.deepStrictEqual(entries.sort(), ['entry.src', 'lib.src']);
  });

  it('should build with the settings of the selected profile', async () => {
    await writeManifest({
      build: { autoCompile: false, installer: { active: false } },
      profiles: { release: { build: { output: 'release' } } }
    });
    await until(() => settings().inspect('transpiler.installer.active')?.workspaceValue === false);
    await settings().update('profile', 'release', vscode.ConfigurationTarget.Workspace);

    try {
      await vscode.commands.executeCommand('greybel.buildFileFromContext', fixture(ENTRY));

      const entries = await waitForFiles(fixture('release'), 2);

      assert.deepStrictEqual(entries.sort(), ['entry.src', 'lib.src']);
      assert.deepStrictEqual(((await readManifest()) as { profiles?: unknown }).profiles, {
        release: { build: { output: 'release' } }
      });
    } finally {
      await settings().update('profile', undefined, vscode.ConfigurationTarget.Workspace);
      await vscode.workspace.fs.delete(fixture('release'), { recursive: true }).then(undefined, () => undefined);
    }
  });
});
