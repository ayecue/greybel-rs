import { expect, it } from 'vitest';

import { openSite } from './support/browser.js';

it('should show the GreyScript reference first', async () => {
  const page = await openSite();

  expect(await page.title()).toBe('GreyScript API');
  expect(await page.locator('.overview h1').textContent()).toBe('GreyScript API – Unofficial Documentation');
  expect(await page.locator('.tab[aria-selected="true"]').textContent()).toBe('API reference');
  expect(await page.locator('.overview h1 img.logo').getAttribute('src')).toBe(
    'https://greybel-assets.fra1.digitaloceanspaces.com/brand/greybel-logo.png'
  );
  expect(await page.locator('link[rel="icon"]').getAttribute('href')).toBe(
    'https://greybel-assets.fra1.digitaloceanspaces.com/brand/favicon.ico'
  );
});

it('should keep the selected tab in the address', async () => {
  const page = await openSite();

  await page.locator('.tab', { hasText: 'Guide' }).click();
  await expect.poll(() => new URL(page.url()).searchParams.get('tab')).toBe('guide');
  await page.reload();
  await page.locator('section.chapter').first().waitFor();

  expect(await page.locator('.tab[aria-selected="true"]').textContent()).toBe('Guide');
  await expect.poll(() => page.locator('section.chapter:visible').count()).toBe(15);
});

it('should filter the reference as its address says', async () => {
  const page = await openSite('?filter=shell.host');

  expect(await page.locator('input[aria-label="Search the API"]').inputValue()).toBe('shell.host');
  await expect.poll(() => page.locator('article.member:visible').count()).toBe(2);
});

it('should show the part of the guide its address links to, and copy the link of a part', async () => {
  const page = await openSite('?tab=guide#guide-values-lists');
  const lists = page.locator('#guide-values-lists');

  await expect.poll(async () => (await lists.boundingBox())?.y ?? -1).toBeGreaterThanOrEqual(0);
  expect((await lists.boundingBox())!.y).toBeLessThan(200);

  const strings = page.locator('#guide-values-strings');

  await strings.scrollIntoViewIfNeeded();
  await strings.hover();
  await strings.locator('a.heading-link').click();

  const copied = new URL(await page.evaluate(() => navigator.clipboard.readText()));

  expect([copied.searchParams.get('tab'), copied.hash]).toEqual(['guide', '#guide-values-strings']);
});
