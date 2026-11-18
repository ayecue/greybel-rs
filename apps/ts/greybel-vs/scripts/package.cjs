// Packages the extension once per VS Code platform given (`node scripts/package.cjs web darwin-arm64 ...`): the `web`
// package carries the workers the browser entry runs, each native package the binaries of its platform. Both entries
// go into every package, as vsce requires each entry package.json declares. What goes in is listed here, handed to
// vsce as its ignore file.
const { execFileSync } = require('node:child_process');
const { mkdirSync, writeFileSync } = require('node:fs');
const { join, resolve } = require('node:path');

const { RUST_TARGETS, buildBinaries, copyBinaries } = require('./build-binaries.cjs');

const ROOT = resolve(__dirname, '..');
const VSCE = require.resolve('@vscode/vsce/vsce');
/** Where README.md's relative links point on the Marketplace and Open VSX pages. */
const CONTENT_URL = 'https://github.com/ayecue/greybel-rs/blob/main/apps/ts/greybel-vs';
const IMAGES_URL = 'https://github.com/ayecue/greybel-rs/raw/main/apps/ts/greybel-vs';

/** Files every package carries; vsce adds package.json itself. */
const SHARED = [
  'README.md',
  'CHANGELOG.md',
  'LICENSE',
  'icon.png',
  'icons/**',
  'snippets.json',
  'schemas/greybel.schema.json',
  'dist/extension.js',
  'dist/extension.browser.js',
  'dist/greyscript.tmLanguage.json',
  'dist/greyscript-signature.tmLanguage.json',
  'dist/greyscriptLanguageConfig.json',
  'dist/api-reference.js',
  'dist/api-reference.wasm',
  'dist/preview.js',
  'dist/preview.wasm'
];
const NATIVE = ['dist/bin/**'];
const WEB = [
  'dist/lsp-web.js',
  'dist/lsp-web.wasm',
  'dist/runtime-web.js',
  'dist/runtime-web.wasm',
  'dist/world-web.js',
  'dist/world-web.wasm',
  'dist/builder-web.js',
  'dist/builder-web.wasm'
];

const PLATFORMS = ['web', ...Object.keys(RUST_TARGETS)];
// `--binaries <folder>` takes each native platform's programs from `<folder>/<platform>` instead of building them.
const args = process.argv.slice(2);
const binariesIndex = args.indexOf('--binaries');
const binaries = binariesIndex === -1 ? undefined : args[binariesIndex + 1];
const targets =
  binariesIndex === -1 ? args : args.filter((_, index) => index !== binariesIndex && index !== binariesIndex + 1);
if (targets.length === 0 || targets.some((target) => !PLATFORMS.includes(target))) {
  throw new Error(`Name the platforms to package: ${PLATFORMS.join(', ')}`);
}

for (const target of targets) {
  const web = target === 'web';
  if (!web && binaries !== undefined) {
    copyBinaries(resolve(binaries, target), target);
  } else if (!web) {
    buildBinaries({ target });
  }

  const ignoreFile = join(ROOT, 'out', 'package', `${target}.vscodeignore`);
  mkdirSync(join(ROOT, 'out', 'package'), { recursive: true });
  writeFileSync(ignoreFile, ['**', ...[...SHARED, ...(web ? WEB : NATIVE)].map((file) => `!${file}`)].join('\n'));

  execFileSync(
    process.execPath,
    [
      VSCE,
      'package',
      '--no-dependencies',
      '--target',
      target,
      '--ignoreFile',
      ignoreFile,
      '--baseContentUrl',
      CONTENT_URL,
      '--baseImagesUrl',
      IMAGES_URL
    ],
    {
      cwd: ROOT,
      stdio: 'inherit'
    }
  );
}
