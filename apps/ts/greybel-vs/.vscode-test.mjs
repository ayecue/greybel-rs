import { defineConfig } from '@vscode/test-cli';

export default defineConfig({
  files: 'out/test/desktop/**/*.test.js',
  workspaceFolder: 'test/e2e/fixture',
  launchArgs: ['--disable-extensions'],
  mocha: { ui: 'bdd', timeout: 60_000 }
});
