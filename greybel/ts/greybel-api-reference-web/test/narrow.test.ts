import { describe, expect, it } from 'vitest';

import { ReferencePage } from './support/reference-page.js';

describe('a narrow API reference', () => {
  it('should list the tabs and the contents in a menu instead of a tab row and a sidebar', async () => {
    const reference = await ReferencePage.open();
    const page = reference.page;

    await page.setViewportSize({ width: 600, height: 800 });

    const toggle = page.locator('button.menu-toggle');

    expect(await page.locator('.toolbar > .tabs').isVisible()).toBe(false);
    expect(await page.locator('#panel-api > nav.contents').isVisible()).toBe(false);
    await toggle.click();
    expect(await toggle.getAttribute('aria-expanded')).toBe('true');

    const menu = page.locator('#reference-menu');

    await menu
      .locator('a', { hasText: /^get_shell$/ })
      .first()
      .click();
    await expect.poll(() => menu.count()).toBe(0);
    await expect
      .poll(() =>
        reference
          .getMember('general', 'get_shell')
          .boundingBox()
          .then((box) => box?.y)
      )
      .toBeLessThan(300);

    await toggle.click();
    await menu.locator('.tab-choice', { hasText: 'Guide' }).click();
    expect(await page.locator('#panel-guide').isVisible()).toBe(true);
    await toggle.click();
    expect(await menu.locator('nav.contents a.entry').first().textContent()).toBe(
      await page.locator('#panel-guide section.chapter h2').first().textContent()
    );

    await page.keyboard.press('Escape');
    await expect.poll(() => menu.count()).toBe(0);
  });

  it('should fit the menu into the visible height', async () => {
    const reference = await ReferencePage.open();
    const page = reference.page;

    await page.setViewportSize({ width: 600, height: 400 });
    await page.locator('button.menu-toggle').click();

    const box = await page.locator('#reference-menu').boundingBox();

    expect((box?.y ?? 0) + (box?.height ?? 0)).toBeLessThanOrEqual(400);
    expect(await page.locator('#reference-menu').evaluate((menu) => menu.scrollHeight > menu.clientHeight)).toBe(true);
  });

  it('should show the tab row and the sidebar again once it is wide', async () => {
    const reference = await ReferencePage.open();
    const page = reference.page;

    expect(await page.locator('.toolbar > .tabs').isVisible()).toBe(true);
    expect(await page.locator('#panel-api > nav.contents').isVisible()).toBe(true);
    expect(await page.locator('button.menu-toggle').isVisible()).toBe(false);
  });
});
