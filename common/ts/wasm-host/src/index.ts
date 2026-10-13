import type { Plugin, UserConfig } from 'vite';

import { CrateBuilder, type CrateBuild, type WasmOptions } from './crate-builder.ts';
import { WASM_KEY } from './worker.ts';

export type { Optimization, WasmOptions } from './crate-builder.ts';

const CRATES = 'crates/';

const CRATE_ID = '\0wasm-host:crate:';
const WORKER_ID = '\0wasm-host:worker:';

export interface WorkerOptions extends WasmOptions {
  readonly crate: string;

  readonly outDir: string;

  readonly fileName?: string;
}

function crateModule({ script }: CrateBuild): string {
  const glue = JSON.stringify(script);

  return [
    `import * as glue from ${glue};`,
    'const eventQueue = [];',
    'let instantiating = false;',
    'function receive(event) {',
    `  const source = event.data?.[${JSON.stringify(WASM_KEY)}];`,
    '  if (source === undefined) {',
    '    eventQueue.push(event);',
    '    return;',
    '  }',
    '  if (instantiating) return;',
    '  instantiating = true;',
    '  glue.default({ module_or_path: source }).then(() => {',
    "    self.removeEventListener('message', receive);",
    '    for (const { data, ports } of eventQueue.splice(0)) {',
    "      self.dispatchEvent(new MessageEvent('message', { data, ports: [...ports] }));",
    '    }',
    '  });',
    '}',
    "self.addEventListener('message', receive);",
    `export * from ${glue};`,
    `export { default } from ${glue};`,
    ''
  ].join('\n');
}

export function wasmCrate(options: WasmOptions): Plugin {
  const builder = new CrateBuilder(options);

  return {
    name: 'wasm-host:crate',
    resolveId(id) {
      if (id.startsWith(CRATES)) return CRATE_ID + id.slice(CRATES.length);
      if (id.startsWith(WORKER_ID)) return id;

      return null;
    },
    async load(id) {
      if (id.startsWith(WORKER_ID)) return `import ${JSON.stringify(CRATES + id.slice(WORKER_ID.length))};\n`;

      if (!id.startsWith(CRATE_ID)) return null;

      const crate = id.slice(CRATE_ID.length);
      const build = await builder.build(crate);

      this.emitFile({ type: 'asset', fileName: `${crate}.wasm`, source: build.wasm });

      return { code: crateModule(build), map: { mappings: '' } };
    }
  };
}

export function wasmWorker(options: WorkerOptions): UserConfig {
  return {
    plugins: [wasmCrate(options)],
    build: {
      outDir: options.outDir,
      target: 'es2022',
      sourcemap: true,
      rollupOptions: {
        input: WORKER_ID + options.crate,
        output: { format: 'iife', entryFileNames: options.fileName ?? 'index.js' }
      }
    }
  };
}
