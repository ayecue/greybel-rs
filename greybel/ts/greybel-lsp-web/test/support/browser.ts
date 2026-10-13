import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { build } from 'esbuild';
import { chromium, type Browser, type Page } from 'playwright';

const WORKER_SCRIPT = new URL('../../dist-test/worker.js', import.meta.url);
const WORKER_WASM = new URL('../../dist-test/greybel-lsp-web.wasm', import.meta.url);

const PAGE_SCRIPT = fileURLToPath(new URL('./page.ts', import.meta.url));

export const LIBRARY_PROJECT = {
  'main.src': '#include "lib.src"\nprint helper\n',
  'lib.src': 'helper = 42\n'
};

export const MISSING_PROJECT = {
  'main.src': 'print missing\n'
};

export const uri = (path: string): string => `file:///project/${path}`;

export class BrowserSession {
  #browser: Browser | undefined;
  #page: Page | undefined;

  async start(): Promise<void> {
    const buildResult = await build({
      entryPoints: [PAGE_SCRIPT],
      bundle: true,
      format: 'iife',
      globalName: 'lspPage',
      platform: 'browser',
      target: 'es2022',
      conditions: ['source'],
      write: false,
      logLevel: 'warning'
    });

    this.#browser = await chromium.launch();
    this.#page = await this.#browser.newPage();
    await this.#page.addScriptTag({ content: buildResult.outputFiles[0]!.text });
    await this.#page.evaluate(([script, wasm]) => lspPage.setWorker(script, wasm), [
      readFileSync(WORKER_SCRIPT, 'utf8'),
      readFileSync(WORKER_WASM).toString('base64')
    ] as const);
  }

  get page(): Page {
    if (this.#page === undefined) throw new Error('the session has not started');

    return this.#page;
  }

  async close(): Promise<void> {
    await this.#browser?.close();
  }
}

export const session = new BrowserSession();
