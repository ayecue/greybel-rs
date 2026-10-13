const esbuild = require('esbuild');
const dotenv = require('dotenv');

const { GREYBEL_DOCUMENTATION_URL = '' } = dotenv.config().parsed ?? {};

/**
 * The webviews' page scripts, each one classic script the webview loads: the output preview's
 * (src/preview/webview.ts, with greyhack-terminal-web's built bundle, whose WebAssembly `pnpm run move` copies to
 * dist/preview.wasm) and the API
 * reference's (src/api-reference/webview.ts, with greybel-api-reference-web's, its WebAssembly in
 * dist/api-reference.wasm, and the documentation website its links copy).
 */
esbuild
  .build({
    entryPoints: {
      preview: './src/preview/webview.ts',
      'api-reference': './src/api-reference/webview.ts'
    },
    bundle: true,
    outdir: 'dist',
    format: 'iife',
    platform: 'browser',
    target: 'ES2022',
    minify: true,
    sourcemap: false,
    define: {
      'process.env.GREYBEL_DOCUMENTATION_URL': JSON.stringify(GREYBEL_DOCUMENTATION_URL)
    }
  })
  .catch((err) => {
    console.error('Failed building the webviews', { err });
    process.exit(1);
  });
