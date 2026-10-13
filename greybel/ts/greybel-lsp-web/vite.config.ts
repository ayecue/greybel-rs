import { wasmWorker } from 'wasm-host';
import { defineConfig } from 'vite';

import { folders } from '../config.ts';

export default defineConfig(({ mode }) =>
  wasmWorker({
    crate: 'greybel-lsp-web',
    outDir: mode === 'test' ? 'dist-test' : 'dist',
    fileName: 'worker.js',
    crates: folders.rust,
    target: folders.target,
    test: mode === 'test'
  })
);
