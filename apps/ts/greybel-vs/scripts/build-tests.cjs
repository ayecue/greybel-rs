const esbuild = require('esbuild');
const { polyfillNode } = require('esbuild-plugin-polyfill-node');

const shared = {
  bundle: true,
  format: 'cjs',
  target: 'ES2022',
  sourcemap: true,
  external: ['vscode']
};

Promise.all([
  esbuild.build({
    ...shared,
    entryPoints: [
      './test/e2e/language-client.test.ts',
      './test/e2e/preview.test.ts',
      './test/e2e/api-reference.test.ts',
      './test/e2e/debug.test.ts',
      './test/e2e/build.test.ts',
      './test/e2e/project.test.ts',
      './test/e2e/setup.test.ts',
      './test/e2e/share.test.ts',
      './test/e2e/terminal.test.ts',
      './test/e2e/world.test.ts'
    ],
    outdir: 'out/test/desktop',
    platform: 'node'
  }),
  esbuild.build({
    ...shared,
    entryPoints: ['./test/e2e/support/web.ts'],
    outfile: 'out/test/web/index.js',
    platform: 'browser',
    plugins: [polyfillNode({ globals: { process: true } })]
  })
]).catch((err) => {
  console.error('Failed building the tests', { err });
  process.exit(1);
});
