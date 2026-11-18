import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { build } from 'esbuild';
import { chromium, type Locator, type Page } from 'playwright';
import { onTestFinished } from 'vitest';

import { MERMAID_URL } from '../../src/components/diagram.js';

const SCRIPT = new URL('../../dist-test/index.iife.js', import.meta.url);
const WASM = new URL('../../dist-test/greybel-api-reference-web.wasm', import.meta.url);
const CLIENT = fileURLToPath(new URL('./client.ts', import.meta.url));

/** The API reference page in Chromium, with its test editor. */
/** Mermaid's files on jsDelivr, which tests serve from the installed `mermaid` package instead. */
const MERMAID_DIST = MERMAID_URL.slice(0, MERMAID_URL.lastIndexOf('/') + 1);
const MERMAID_FILES = new URL('../../node_modules/mermaid/dist/', import.meta.url);

export class ReferencePage {
  readonly page: Page;

  #pageScript: Promise<string> | undefined;

  constructor(page: Page) {
    this.page = page;
  }

  static async open(repositoryUrl?: string): Promise<ReferencePage> {
    const browser = await chromium.launch();

    onTestFinished(() => browser.close());

    const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
    const reference = new ReferencePage(page);

    await page.route(`${MERMAID_DIST}**`, (route) =>
      route.fulfill({
        contentType: 'text/javascript',
        body: readFileSync(new URL(route.request().url().slice(MERMAID_DIST.length), MERMAID_FILES))
      })
    );

    await reference.#load(repositoryUrl);

    return reference;
  }

  getSection(name: string): Locator {
    return this.page.locator('section.section', {
      has: this.page.locator('h2', { hasText: new RegExp(`^${name}( |$)`) })
    });
  }

  getMember(owner: string, name: string): Locator {
    return this.getSection(owner).locator('article.member', {
      has: this.page.locator('h3 .name', { hasText: new RegExp(`^${name}$`) })
    });
  }

  getChapter(title: string): Locator {
    return this.page.locator('section.chapter', {
      has: this.page.locator('h2', { hasText: new RegExp(`^${title}$`) })
    });
  }

  /** Selects the guide's tab, once its chapters are available. */
  async openGuide(): Promise<void> {
    await this.page.locator('.tab', { hasText: 'Guide' }).click();
    await this.page.locator('section.chapter').first().waitFor();
  }

  getOpenedDocument(): Promise<{ content: string }> {
    return this.page.evaluate(() => referencePage.client.getOpenedDocument());
  }

  async #load(repositoryUrl: string | undefined): Promise<void> {
    await this.page.setContent('<body style="margin:0"><div id="reference"></div></body>');
    await this.page.addScriptTag({ content: readFileSync(SCRIPT, 'utf8') });
    await this.page.addScriptTag({ content: await this.#getPageScript() });
    await this.page.evaluate(([wasm, url]) => referencePage.client.open(wasm!, url), [
      readFileSync(WASM).toString('base64'),
      repositoryUrl
    ] as const);
  }

  #getPageScript(): Promise<string> {
    this.#pageScript ??= build({
      entryPoints: [CLIENT],
      bundle: true,
      format: 'iife',
      globalName: 'referencePage',
      platform: 'browser',
      target: 'es2022',
      conditions: ['source'],
      write: false,
      logLevel: 'warning'
    }).then((result) => result.outputFiles[0]!.text);

    return this.#pageScript;
  }
}
