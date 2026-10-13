// Builds the Rust programs the desktop extension runs into dist/bin: the language server, the runtime, the game server
// whose world the runtimes share, and the builder.
// `--debug` builds them without optimizations, for tests. `--target <platform>` builds them for a VS Code platform
// other than this machine's (see RUST_TARGETS), which needs that Rust target and its linker installed.
const { execFileSync } = require('node:child_process');
const { copyFileSync, mkdirSync, rmSync } = require('node:fs');
const { join, resolve } = require('node:path');

const RUST_WORKSPACE = resolve(__dirname, '../../../rs');
const TARGET = process.env.CARGO_TARGET_DIR ?? resolve(__dirname, '../../../../target');
const OUTPUT = resolve(__dirname, '../dist/bin');
const BINARIES = [
  { pkg: 'greybel-lsp-stdio', name: 'greybel-lsp' },
  { pkg: 'greybel-runtime-stdio', name: 'greybel-runtime' },
  { pkg: 'greyhack-world-server', name: 'greyhack-world-server' },
  { pkg: 'greybel-builder-stdio', name: 'greybel-builder' }
];

/** The Rust target each native VS Code platform runs. */
const RUST_TARGETS = {
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'linux-armhf': 'armv7-unknown-linux-gnueabihf',
  'alpine-x64': 'x86_64-unknown-linux-musl',
  'alpine-arm64': 'aarch64-unknown-linux-musl',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin'
};

/** Replaces dist/bin with the binaries for `target` (a key of RUST_TARGETS), or for this machine without one. */
function buildBinaries({ debug = false, target } = {}) {
  const rustTarget = target === undefined ? undefined : RUST_TARGETS[target];
  if (target !== undefined && rustTarget === undefined) {
    throw new Error(`'${target}' is not a native VS Code platform: ${Object.keys(RUST_TARGETS).join(', ')}`);
  }

  const packages = BINARIES.flatMap(({ pkg }) => ['--package', pkg]);
  execFileSync(
    'cargo',
    [
      'build',
      '--locked',
      ...(debug ? [] : ['--release']),
      ...(rustTarget ? ['--target', rustTarget] : []),
      ...packages
    ],
    { cwd: RUST_WORKSPACE, stdio: 'inherit' }
  );

  const windows = target === undefined ? process.platform === 'win32' : target.startsWith('win32-');
  const extension = windows ? '.exe' : '';
  const built = join(TARGET, ...(rustTarget ? [rustTarget] : []), debug ? 'debug' : 'release');
  rmSync(OUTPUT, { recursive: true, force: true });
  mkdirSync(OUTPUT, { recursive: true });
  for (const { name } of BINARIES) {
    copyFileSync(join(built, name + extension), join(OUTPUT, name + extension));
  }
}

module.exports = { RUST_TARGETS, buildBinaries };

if (require.main === module) {
  const args = process.argv.slice(2);
  const target = args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined;
  buildBinaries({ debug: args.includes('--debug'), target });
}
