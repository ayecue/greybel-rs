import { fileURLToPath } from 'node:url';

import { chromium, type Page } from 'playwright';
import { preview, type PreviewServer } from 'vite';
import { onTestFinished } from 'vitest';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

let server: Promise<PreviewServer> | undefined;

/** The site built for the tests (`vite build --mode test`), served once for every test. */
function served(): Promise<PreviewServer> {
  server ??= preview({ root: ROOT, mode: 'test', logLevel: 'warn', preview: { port: 0, open: false } });

  return server;
}

/** The site at `query`, once its reference shows its sections. */
export async function openSite(query = ''): Promise<Page> {
  const url = (await served()).resolvedUrls?.local[0];

  if (url === undefined) throw new Error('the preview server has no address');

  const browser = await chromium.launch();

  onTestFinished(() => browser.close());

  const page = await browser.newPage({
    viewport: { width: 1200, height: 800 },
    permissions: ['clipboard-read', 'clipboard-write']
  });

  page.on('pageerror', (error) => console.error('page error:', error));
  await page.goto(`${url}${query}`);
  await page.locator('section.section').first().waitFor({ state: 'attached' });

  return page;
}
