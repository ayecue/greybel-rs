import { defineConfig } from '@vscode/test-cli';
import { mkdtempSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

export default defineConfig({
  files: 'out/test/desktop/**/*.test.js',
  workspaceFolder: 'test/e2e/fixture',
  // Software WebGL2 (SwiftShader) for the terminal webview, as on CI's GPU-less runners, where Chromium blocklists
  // WebGL2 otherwise. `=true` keeps VS Code from reading the workspace folder after it as the switch's value.
  launchArgs: ['--disable-extensions', '--disable-gpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader=true'],
  // Isolate test world saves from the developer's saved worlds.
  env: { GREYBEL_WORLDS: mkdtempSync(join(tmpdir(), 'greybel-worlds-')) },
  mocha: { ui: 'bdd', timeout: 60_000 }
});
