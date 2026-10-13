import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { build } from 'esbuild';
import { chromium, type Page } from 'playwright';
import { onTestFinished } from 'vitest';

const SCRIPT = new URL('../../dist-test/index.iife.js', import.meta.url);
const WASM = new URL('../../dist-test/greybel-api-reference-web.wasm', import.meta.url);

const PAGE = fileURLToPath(new URL('./page.ts', import.meta.url));

let pageScript: Promise<string> | undefined;

function bundledPage(): Promise<string> {
  pageScript ??= build({
    entryPoints: [PAGE],
    bundle: true,
    format: 'iife',
    globalName: 'referencePage',
    platform: 'browser',
    target: 'es2022',
    conditions: ['source'],
    write: false,
    logLevel: 'warning'
  }).then((bundled) => bundled.outputFiles[0]!.text);

  return pageScript;
}

export async function openPage(): Promise<Page> {
  const browser = await chromium.launch();

  onTestFinished(() => browser.close());

  const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });

  await page.setContent('<body style="margin:0"><div id="reference"></div></body>');
  await page.addScriptTag({ content: readFileSync(SCRIPT, 'utf8') });
  await page.addScriptTag({ content: await bundledPage() });
  await page.evaluate((wasm) => referencePage.open(wasm), readFileSync(WASM).toString('base64'));

  return page;
}
