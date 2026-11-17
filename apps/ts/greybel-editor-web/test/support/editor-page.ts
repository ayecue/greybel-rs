import { fileURLToPath } from 'node:url';

import { chromium, type Locator, type Page } from 'playwright';
import { preview, type PreviewServer } from 'vite';
import { expect, onTestFinished } from 'vitest';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

/** The editor page in Chromium, served from the test build (`vite build --mode test`). */
export class EditorPage {
  static #server: Promise<PreviewServer> | undefined;

  readonly page: Page;

  private constructor(page: Page) {
    this.page = page;
  }

  /** Opens the project `text` encodes. `prepare` runs before navigation, e.g. to route requests. */
  static open(text: string, prepare?: (page: Page) => Promise<void>): Promise<EditorPage> {
    return EditorPage.openWithQuery(new URLSearchParams({ c: btoa(encodeURIComponent(text)) }), prepare);
  }

  /** Opens the page with `query`, such as `?id=` of a saved project. */
  static async openWithQuery(query: URLSearchParams, prepare?: (page: Page) => Promise<void>): Promise<EditorPage> {
    EditorPage.#server ??= preview({ root: ROOT, mode: 'test', logLevel: 'warn', preview: { port: 0, open: false } });

    const url = (await EditorPage.#server).resolvedUrls?.local[0];

    if (url === undefined) throw new Error('the preview server has no address');

    const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });

    onTestFinished(() => browser.close());

    const page = await browser.newPage({
      viewport: { width: 1400, height: 900 },
      permissions: ['clipboard-read', 'clipboard-write']
    });

    page.on('dialog', (dialog) => void dialog.accept());
    page.on('pageerror', (error) => console.error('page error:', error));
    await prepare?.(page);
    await page.goto(`${url}?${query.toString()}`);
    await page.locator('.cm-content').waitFor();

    return new EditorPage(page);
  }

  /** The text of a project document, as a share link carries it. */
  static encodeProject(entry: string, files: Readonly<Record<string, string>>): string {
    return JSON.stringify({ greybel: 'project', version: 1, entry, files });
  }

  /** Waits until the debugger shows a stop. */
  async waitForStop(): Promise<void> {
    await this.page.locator('.debug-panel.stopped').waitFor();
  }

  async getTerminalText(): Promise<string> {
    return (await this.page.locator('.terminal-text').textContent()) ?? '';
  }

  async clickBreakpointGutter(line: number): Promise<void> {
    const gutter = await this.page.locator('.cm-breakpoint-gutter').boundingBox();
    const row = await this.page
      .locator('.cm-line')
      .nth(line - 1)
      .boundingBox();

    if (gutter === null || row === null) throw new Error(`the editor shows no line ${line}`);
    await this.page.mouse.click(gutter.x + gutter.width / 2, row.y + row.height / 2);
  }

  getFileEntry(path: string): Locator {
    return this.page.locator(`.file-list li[data-path="${path}"]`);
  }

  async getShownFile(): Promise<string | null> {
    const paths = await this.page
      .locator('.file-list li.active')
      .evaluateAll((entries) => entries.map((entry) => entry.getAttribute('data-path')));

    return paths[0] ?? null;
  }

  async openFile(path: string): Promise<void> {
    await this.getFileEntry(path).locator('.file-path').click();
  }

  /** Replaces the code of the file shown. */
  async typeCode(code: string): Promise<void> {
    await this.page.locator('.cm-content').click();
    await this.page.keyboard.press('ControlOrMeta+A');
    await this.page.keyboard.press('Delete');
    await this.page.keyboard.insertText(code);
  }

  async hoverCode(text: string): Promise<void> {
    const point = await this.page.locator('.cm-content').evaluate((content, target) => {
      const walker = document.createTreeWalker(content, NodeFilter.SHOW_TEXT);

      for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
        const index = node.textContent?.indexOf(target) ?? -1;

        if (index < 0) continue;

        const range = document.createRange();

        range.setStart(node, index);
        range.setEnd(node, index + target.length);

        const box = range.getBoundingClientRect();

        return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
      }

      return undefined;
    }, text);

    if (point === undefined) throw new Error(`the code shows no ${text}`);
    await this.page.mouse.move(point.x, point.y);
  }

  getTab(path: string): Locator {
    return this.page.locator(`.tab-bar .tab[data-path="${path}"]`);
  }

  getTabPaths(): Promise<string[]> {
    return this.page
      .locator('.tab-bar .tab')
      .evaluateAll((tabs) => tabs.map((element) => element.getAttribute('data-path') ?? ''));
  }

  getTerminalTab(name: string): Locator {
    return this.page.locator('.terminal-tab', { has: this.page.locator('.terminal-name', { hasText: name }) });
  }

  async selectProfile(profile: string): Promise<void> {
    await this.page.locator('select.profile').selectOption(profile);
  }

  /** Runs the selected terminal's script with `profile` until it ends. */
  async runToEnd(profile: string): Promise<void> {
    await this.selectProfile(profile);
    await this.page.locator('button.run').click();
    await expect.poll(() => this.page.locator('button.run').isEnabled(), { timeout: 10_000 }).toBe(true);
  }

  getSplitter(name: string): Locator {
    return this.page.locator(`.splitter[data-name="${name}"]`);
  }

  async dragSplitter(name: string, x: number, y: number): Promise<void> {
    const box = await this.getSplitter(name).boundingBox();

    if (box === null) throw new Error(`the page shows no splitter ${name}`);

    const start = { x: box.x + box.width / 2, y: box.y + box.height / 2 };

    await this.page.mouse.move(start.x, start.y);
    await this.page.mouse.down();
    await this.page.mouse.move(start.x + x, start.y + y, { steps: 4 });
    await this.page.mouse.up();
  }
}
