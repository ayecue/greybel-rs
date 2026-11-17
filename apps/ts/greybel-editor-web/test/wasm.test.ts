import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';

describe('the loading of the WebAssembly modules', () => {
  it('should show the parts still loading and wait with running until the runtime is loaded', async () => {
    let release = () => {};

    const isReleased = new Promise<void>((resolve) => {
      release = resolve;
    });
    const editor = await EditorPage.open('print("ran")', async (page) => {
      await page.route('**/greybel-runtime-web*.wasm', async (route) => {
        await isReleased;
        await route.continue();
      });
    });
    const { page } = editor;
    const status = page.locator('.loading-status');

    await expect.poll(() => status.textContent()).toContain('runtime');
    expect(await page.locator('button.run').isDisabled()).toBe(true);

    release();
    await expect.poll(() => status.count()).toBe(0);
    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('ran');
  });

  it('should work after downloads longer than the request and connection timeouts', async () => {
    const editor = await EditorPage.open('print("ran")', async (page) => {
      await page.route('**/*.wasm', async (route) => {
        await new Promise((resolve) => setTimeout(resolve, 25_000));
        await route.continue();
      });
    });
    const { page } = editor;

    await expect.poll(() => page.locator('.cm-token-function').count(), { timeout: 45_000 }).toBeGreaterThan(0);
    await page.locator('button.run').click({ timeout: 45_000 });
    await expect.poll(() => editor.getTerminalText()).toContain('ran');
    await page.locator('button.transform').click();
    await expect.poll(() => page.locator('.transform-result').inputValue()).toContain('print');
    expect(await page.locator('.loading-failure').count()).toBe(0);
  }, 90_000);

  it('should show a part that failed to load', async () => {
    const editor = await EditorPage.open('print(1)', async (page) => {
      await page.route('**/greybel-builder-web*.wasm', (route) => route.fulfill({ status: 404, body: 'missing' }));
    });
    const { page } = editor;

    await expect
      .poll(() => page.locator('.loading-failure').textContent())
      .toBe("Couldn't load the transformer: the server responded 404 Not Found");
    await expect.poll(() => page.locator('.loading-status').count()).toBe(0);
    expect(await page.locator('.loading-failure').count()).toBe(1);
    expect(await page.locator('button.transform').isDisabled()).toBe(true);
  });
});
