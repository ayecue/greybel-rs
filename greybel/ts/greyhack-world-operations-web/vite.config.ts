import { wasmWorker } from 'wasm-host';
import { defineConfig } from 'vite';

import { folders } from '../config.ts';

export default defineConfig(
  wasmWorker({
    crate: 'greyhack-world-operations-web',
    outDir: 'dist',
    crates: folders.rust,
    target: folders.target,
    test: false,
    optimization: 'speed'
  })
);
