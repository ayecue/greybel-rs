import { basename } from 'node:path';

import { defineConfig } from 'vite';

/** Set by CI: the folder on DigitalOcean Spaces the production build's `.wasm` files are uploaded to. */
const WASM_BASE_URL = process.env.WASM_BASE_URL?.replace(/\/$/, '');

export default defineConfig(({ mode }) => ({
  base: './',
  experimental: {
    renderBuiltUrl: (filename: string) =>
      WASM_BASE_URL !== undefined && mode !== 'test' && filename.endsWith('.wasm')
        ? `${WASM_BASE_URL}/${basename(filename)}`
        : { relative: true }
  },
  build: {
    outDir: mode === 'test' ? 'dist-test' : 'dist',
    target: 'es2022',
    sourcemap: true
  }
}));
