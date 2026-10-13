import type { Locator, Page } from 'playwright';

export function section(page: Page, name: string): Locator {
  return page.locator('section.section', { has: page.locator('h2', { hasText: new RegExp(`^${name}( |$)`) }) });
}

export function member(page: Page, owner: string, name: string): Locator {
  return section(page, owner).locator('article.member', {
    has: page.locator('h3 .name', { hasText: new RegExp(`^${name}$`) })
  });
}

export function chapter(page: Page, title: string): Locator {
  return page.locator('section.chapter', { has: page.locator('h2', { hasText: new RegExp(`^${title}$`) }) });
}

/** Selects the guide's tab, once its chapters are rendered. */
export async function openGuide(page: Page): Promise<void> {
  await page.locator('.tab', { hasText: 'Guide' }).click();
  await page.locator('section.chapter').first().waitFor();
}
