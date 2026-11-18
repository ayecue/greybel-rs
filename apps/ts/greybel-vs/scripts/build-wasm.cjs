// Builds the WebAssembly packages the web extension copies into dist (see the move:* scripts): the language server, the
// runtime, the game server whose world the runtimes share, the builder, the terminal preview and the API reference.
// The web tests run them from dist, so a package left unbuilt would test its last build instead of the current code.
const { execFileSync } = require('node:child_process');
const { resolve } = require('node:path');

const PACKAGES = resolve(__dirname, '../../../../greybel/ts');
const WASM_PACKAGES = [
  'greybel-lsp-web',
  'greybel-runtime-web',
  'greyhack-world-operations-web',
  'greybel-builder-web',
  'greyhack-terminal-web',
  'greybel-api-reference-web'
];

for (const name of WASM_PACKAGES) {
  execFileSync('pnpm', ['--dir', resolve(PACKAGES, name), 'build'], { stdio: 'inherit' });
}
