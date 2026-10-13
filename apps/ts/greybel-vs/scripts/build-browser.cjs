const esbuild = require('esbuild');
const dotenv = require('dotenv');
const envObj = dotenv.config().parsed;

const build = async () => {
  try {
    await esbuild.build({
      entryPoints: ['./out/extension-browser.js'],
      bundle: true,
      outfile: 'dist/extension.browser.js',
      globalName: 'greyscript',
      sourcemap: false,
      minify: true,
      minifyWhitespace: true,
      minifyIdentifiers: true,
      minifySyntax: true,
      target: 'ESNext',
      platform: 'browser',
      treeShaking: true,
      format: 'cjs',
      external: ['vscode'],
      // @vscode/debugadapter's only Node modules: `events` is the npm package, `url` the global URL.
      alias: { url: './scripts/browser/url.cjs' },
      define: {
        // No `process` in the browser: environment lookups such as ansi-escapes' read nothing.
        'process.env': '{}',
        'process.env.NODE_ENV': '"production"',
        'process.platform': '"browser"',
        ...Object.entries(envObj).reduce((result, [key, value]) => {
          result[`process.env.${key}`] = `"${value}"`;
          return result;
        }, {})
      }
    });
  } catch (err) {
    console.error('Failed building project', { err });
    process.exit(1);
  }
};

build();
