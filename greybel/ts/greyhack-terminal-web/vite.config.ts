import { fileURLToPath } from 'node:url';

import { wasmCrate } from 'wasm-host';
import { defineConfig } from 'vite';

import { folders } from '../config.ts';

export default defineConfig(({ mode }) => ({
  resolve: { alias: { 'text-mesh-shaders': fileURLToPath(folders.shaders) } },
  worker: {
    format: 'iife',
    plugins: () => [wasmCrate({ crates: folders.rust, target: folders.target, test: mode === 'test' })]
  },
  build: {
    outDir: mode === 'test' ? 'dist-test' : 'dist',
    target: 'es2022',
    sourcemap: true,
    lib: {
      entry: 'src/index.ts',
      formats: ['es', 'iife'],
      name: 'greyhackTerminalWeb',
      fileName: (format) => (format === 'es' ? 'index.js' : `index.${format}.js`)
    }
  }
}));
