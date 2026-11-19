import * as assert from 'assert';
import { existsSync } from 'fs';
import * as vscode from 'vscode';

import { FakeGame, WindowStub } from './support/setup';
import { TIMEOUT, waitFor } from './support/workspace';

describe('setting up Grey Hack', function () {
  this.timeout(TIMEOUT);

  it('should review and remove the message hook of the game in GREY_HACK_PATH', async function () {
    if (vscode.env.uiKind === vscode.UIKind.Web) this.skip();

    const game = new FakeGame(['BepInEx/core/BepInEx.dll', 'BepInEx/plugins/GreyHackMessageHook5.dll']);
    const window = new WindowStub('Remove the message hook', 'Apply');

    try {
      await vscode.commands.executeCommand('greybel.setupGame');
      await waitFor('the result of the setup', () => window.information.length === 1);

      assert.strictEqual(
        window.placeHolder,
        'BepInEx 5, GreyHackMessageHook5, port 7777: Setup is done. The message hook connects on port 7777.'
      );
      assert.deepStrictEqual(window.reviews, ['• Remove the message hook']);
      assert.ok(window.information[0]!.startsWith("Not ready: the message hook isn't installed."));
      assert.ok(!existsSync(game.path('BepInEx/plugins/GreyHackMessageHook5.dll')));
    } finally {
      window.restore();
      game.remove();
    }
  });
});
