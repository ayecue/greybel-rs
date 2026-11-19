import type { Locator } from 'playwright';
import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';

async function getWidth(locator: Locator): Promise<number> {
  return (await locator.boundingBox())?.width ?? 0;
}

async function getHeight(locator: Locator): Promise<number> {
  return (await locator.boundingBox())?.height ?? 0;
}

describe('the parts of the page', () => {
  it('should be resized by dragging a splitter, for the next visit too, until a double click resets it', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;
    const files = page.locator('.file-list');

    expect(await getWidth(files)).toBe(200);
    await editor.dragSplitter('file-list', 100, 0);
    expect(await getWidth(files)).toBe(300);

    await page.reload();
    await page.locator('.cm-content').waitFor();
    expect(await getWidth(files)).toBe(300);

    await editor.getSplitter('file-list').dblclick();
    expect(await getWidth(files)).toBe(200);
  });

  it('should keep both neighbours of a splitter at their least size', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    await editor.dragSplitter('file-list', -500, 0);

    expect(await getWidth(page.locator('.file-list'))).toBe(120);
  });

  it('should show a link to star the repository in the toolbar', async () => {
    const editor = await EditorPage.open('print(1)');
    const link = editor.page.locator('.toolbar a.star-link');

    expect(await link.textContent()).toBe('★ Star on GitHub');
    expect(await link.getAttribute('href')).toBe('https://github.com/ayecue/greybel-rs');
    expect(await link.getAttribute('target')).toBe('_blank');
  });

  it('should show the Greybel logo in the toolbar and as the icon of the page', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    expect(await page.locator('.toolbar h1').textContent()).toBe('Greybel Editor');
    expect(await page.locator('.toolbar h1 img.logo').getAttribute('src')).toBe(
      'https://greybel-assets.fra1.digitaloceanspaces.com/brand/greybel-logo.png'
    );
    expect(await page.locator('link[rel="icon"]').getAttribute('href')).toBe(
      'https://greybel-assets.fra1.digitaloceanspaces.com/brand/favicon.ico'
    );
  });

  it("should fit the page's width with the API reference open", async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    await page.locator('button.reference-toggle').click();
    await page.locator('.reference-panel').waitFor();

    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(1400);
    expect(await editor.getSplitter('reference').isVisible()).toBe(true);
  });

  it("should fit the API reference's menu into its panel", async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    await page.setViewportSize({ width: 1400, height: 500 });
    await page.locator('button.reference-toggle').click();
    await page.locator('.reference-panel button.menu-toggle').click();

    const menu = await page.locator('#reference-menu').boundingBox();
    const panel = await page.locator('.reference-panel').boundingBox();

    expect((menu?.y ?? 0) + (menu?.height ?? 0)).toBeLessThanOrEqual((panel?.y ?? 0) + (panel?.height ?? 0));
  });

  it('should be resized with the arrow keys on a splitter', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;
    const panels = page.locator('.panels');
    const before = await getHeight(panels);

    await editor.getSplitter('panels').focus();
    await page.keyboard.press('ArrowUp');

    expect(await getHeight(panels)).toBe(Math.round(before) + 16);
  });

  it('should resize the transform panel against the execute panel', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;
    const transform = page.locator('.transform-panel');
    const before = await getWidth(transform);

    await editor.dragSplitter('transform', 50, 0);

    expect(await getWidth(transform)).toBe(Math.round(before + 50));
  });
});
