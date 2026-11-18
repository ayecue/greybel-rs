import { defineConfig } from '@vscode/test-cli';
import { mkdtempSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

export default defineConfig({
  files: 'out/test/desktop/**/*.test.js',
  workspaceFolder: 'test/e2e/fixture',
  launchArgs: ['--disable-extensions'],
  // Isolate test world saves from the developer's saved worlds.
  env: { GREYBEL_WORLDS: mkdtempSync(join(tmpdir(), 'greybel-worlds-')) },
  mocha: { ui: 'bdd', timeout: 60_000 }
});
