import { fileURLToPath } from 'node:url';

import { chromium, type Locator, type Page } from 'playwright';
import { preview, type PreviewServer } from 'vite';
import { onTestFinished } from 'vitest';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

let server: Promise<PreviewServer> | undefined;

/** The page built for the tests (`vite build --mode test`), served once for every test. */
function served(): Promise<PreviewServer> {
  server ??= preview({ root: ROOT, mode: 'test', logLevel: 'warn', preview: { port: 0, open: false } });

  return server;
}

/** The editor opened with the project `text` encodes, its language server started. */
export async function openEditor(text: string): Promise<Page> {
  const url = (await served()).resolvedUrls?.local[0];

  if (url === undefined) throw new Error('the preview server has no address');

  const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });

  onTestFinished(() => browser.close());

  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const query = new URLSearchParams({ c: btoa(encodeURIComponent(text)) });

  page.on('dialog', (dialog) => void dialog.accept());

  page.on('pageerror', (error) => console.error('page error:', error));
  await page.goto(`${url}?${query.toString()}`);
  await page.locator('.cm-content').waitFor();

  return page;
}

/** The text the program's terminal shows. */
export function terminalText(page: Page): Promise<string> {
  return page
    .locator('.terminal-text')
    .textContent()
    .then((text) => text ?? '');
}

/** Clicks the breakpoint gutter beside `line`, starting at 1. */
export async function clickBreakpointGutter(page: Page, line: number): Promise<void> {
  const gutter = await page.locator('.cm-breakpoint-gutter').boundingBox();
  const row = await page
    .locator('.cm-line')
    .nth(line - 1)
    .boundingBox();

  if (gutter === null || row === null) throw new Error(`the editor shows no line ${line}`);
  await page.mouse.click(gutter.x + gutter.width / 2, row.y + row.height / 2);
}

/** The text of a project of `files` by path, its runs starting at `entry`. */
export function projectText(entry: string, files: Readonly<Record<string, string>>): string {
  return JSON.stringify({ greybel: 'project', version: 1, entry, files });
}

/** The entry of the file list for the file at `path`. */
export function fileEntry(page: Page, path: string): Locator {
  return page.locator(`.file-list li[data-path="${path}"]`);
}

/** The path of the file the editor shows; none when it shows none. */
export async function shownFile(page: Page): Promise<string | null> {
  const paths = await page
    .locator('.file-list li.active')
    .evaluateAll((entries) => entries.map((entry) => entry.getAttribute('data-path')));

  return paths[0] ?? null;
}

/** Replaces the code of the file shown with `code`, typed. */
export async function typeCode(page: Page, code: string): Promise<void> {
  await page.locator('.cm-content').click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('Delete');
  await page.keyboard.insertText(code);
}

/** The tab of the file at `path`. */
export function tab(page: Page, path: string): Locator {
  return page.locator(`.tab-bar .tab[data-path="${path}"]`);
}

/** The paths of the files with a tab, in order. */
export function tabPaths(page: Page): Promise<string[]> {
  return page
    .locator('.tab-bar .tab')
    .evaluateAll((tabs) => tabs.map((element) => element.getAttribute('data-path') ?? ''));
}

/** Shows the file at `path` by a click in the file list. */
export async function openFile(page: Page, path: string): Promise<void> {
  await fileEntry(page, path).locator('.file-path').click();
}
