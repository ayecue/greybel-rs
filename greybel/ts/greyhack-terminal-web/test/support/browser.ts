import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { build } from 'esbuild';
import { chromium, type Page } from 'playwright';
import { inject, onTestFinished } from 'vitest';

const SCRIPT = new URL('../../dist-test/index.iife.js', import.meta.url);
const WASM = new URL('../../dist-test/greyhack-terminal-web.wasm', import.meta.url);
const FIXTURES = 'greyhack-terminal-web/tests/fixtures';

const PAGE = fileURLToPath(new URL('./page.ts', import.meta.url));

export interface RecordedCase {
  readonly source: string;
  readonly height: number;
  readonly capture: string;
}

export const WIDTH = 1000;

export function recordedCase(index: number): RecordedCase {
  const root = `${inject('rust')}/${FIXTURES}`;
  const fixture = JSON.parse(readFileSync(`${root}/tmp-meshes.json`, 'utf8')) as {
    cases: { source: string; image: string; height: number }[];
  };
  const { source, image, height } = fixture.cases[index]!;

  return { source, height, capture: readFileSync(`${root}/captures/${image}`).toString('base64') };
}

let pageScript: Promise<string> | undefined;

function bundledPage(): Promise<string> {
  pageScript ??= build({
    entryPoints: [PAGE],
    bundle: true,
    format: 'iife',
    globalName: 'terminalPage',
    platform: 'browser',
    target: 'es2022',
    conditions: ['source'],
    write: false,
    logLevel: 'warning'
  }).then((bundled) => bundled.outputFiles[0]!.text);

  return pageScript;
}

export async function openPage(width: number, height: number): Promise<Page> {
  const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });

  onTestFinished(() => browser.close());

  const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });

  await page.setContent(
    `<body style="margin:0"><canvas id="terminal" style="display:block;width:${width}px;height:${height}px"></canvas></body>`
  );
  await page.addScriptTag({ content: readFileSync(SCRIPT, 'utf8') });
  await page.addScriptTag({ content: await bundledPage() });
  await page.evaluate((wasm) => terminalPage.open(wasm), readFileSync(WASM).toString('base64'));

  return page;
}

export async function screenshot(page: Page): Promise<string> {
  return (await page.locator('#terminal').screenshot()).toString('base64');
}
