// Builds the Rust programs the desktop extension runs into dist/bin: the language server, the runtime, the game server
// whose world the runtimes share, the builder, and greyhack-setup without its window.
// `--debug` builds them without optimizations, for tests. `--target <platform>` builds them for a VS Code platform
// other than this machine's (see RUST_TARGETS), which needs that Rust target and its linker installed.
const { execFileSync } = require('node:child_process');
const { chmodSync, copyFileSync, mkdirSync, rmSync } = require('node:fs');
const { join, resolve } = require('node:path');

const RUST_WORKSPACE = resolve(__dirname, '../../../rs');
const TARGET = process.env.CARGO_TARGET_DIR ?? resolve(__dirname, '../../../../target');
const OUTPUT = resolve(__dirname, '../dist/bin');
const BINARIES = [
  { pkg: 'greybel-lsp-stdio', name: 'greybel-lsp' },
  { pkg: 'greybel-runtime-stdio', name: 'greybel-runtime' },
  { pkg: 'greyhack-world-server', name: 'greyhack-world-server' },
  { pkg: 'greybel-builder-stdio', name: 'greybel-builder' },
  { pkg: 'greyhack-setup-wizard', name: 'greyhack-setup', features: ['--no-default-features'] }
];

/** The Rust target each native VS Code platform runs. */
const RUST_TARGETS = {
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
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

  // Packages with their own feature flags build separately: Cargo applies --no-default-features to every named package.
  const groups = [
    BINARIES.filter(({ features }) => features === undefined).flatMap(({ pkg }) => ['--package', pkg]),
    ...BINARIES.filter(({ features }) => features !== undefined).map(({ pkg, features }) => [
      '--package',
      pkg,
      ...features
    ])
  ];
  for (const group of groups) {
    execFileSync(
      'cargo',
      ['build', '--locked', ...(debug ? [] : ['--release']), ...(rustTarget ? ['--target', rustTarget] : []), ...group],
      { cwd: RUST_WORKSPACE, stdio: 'inherit' }
    );
  }

  copyBinaries(join(TARGET, ...(rustTarget ? [rustTarget] : []), debug ? 'debug' : 'release'), target);
}

/** Replaces dist/bin with the binaries in `folder`, built for `target` (a key of RUST_TARGETS) or this machine. */
function copyBinaries(folder, target) {
  const windows = target === undefined ? process.platform === 'win32' : target.startsWith('win32-');
  const extension = windows ? '.exe' : '';
  rmSync(OUTPUT, { recursive: true, force: true });
  mkdirSync(OUTPUT, { recursive: true });
  for (const { name } of BINARIES) {
    copyFileSync(join(folder, name + extension), join(OUTPUT, name + extension));
    chmodSync(join(OUTPUT, name + extension), 0o755);
  }
}

module.exports = { BINARIES, RUST_TARGETS, buildBinaries, copyBinaries };

if (require.main === module) {
  const args = process.argv.slice(2);
  const target = args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined;
  buildBinaries({ debug: args.includes('--debug'), target });
}
