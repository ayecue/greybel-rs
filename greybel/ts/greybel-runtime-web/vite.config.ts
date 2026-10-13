import { wasmWorker } from 'wasm-host';
import { defineConfig } from 'vite';

import { folders } from '../config.ts';

export default defineConfig(
  wasmWorker({
    crate: 'greybel-runtime-web',
    outDir: 'dist',
    crates: folders.rust,
    target: folders.target,
    test: false,
    optimization: 'speed'
  })
);
