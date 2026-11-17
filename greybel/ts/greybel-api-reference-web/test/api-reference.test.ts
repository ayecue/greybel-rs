import { describe, expect, it } from 'vitest';

import { openPage } from './support/browser.js';
import { chapter, member, openGuide, section } from './support/locators.js';

describe('the API reference', () => {
  it('should show every type of the table of contents with its members', async () => {
    const page = await openPage();

    const sections = await page.locator('section.section h2').count();
    const entries = await page.locator('#panel-api nav.contents > ul > li').count();

    expect(sections).toBeGreaterThan(25);
    expect(sections).toBe(entries);
    expect(await member(page, 'shell', 'host_computer').locator('h3').textContent()).toBe('host_computer(): computer');
    expect(await section(page, 'shell').locator('h2').textContent()).toBe('shell extends map');
  });

  it('should show only what the search matches', async () => {
    const page = await openPage();

    await page.locator('input[aria-label="Search the API"]').fill('shell.host');

    await expect.poll(() => page.locator('article.member:visible').count()).toBe(2);
    expect(await member(page, 'ftpShell', 'host_computer').isVisible()).toBe(true);
    expect(await section(page, 'general').isVisible()).toBe(false);
  });

  it('should hide the members of an excluded tag', async () => {
    const page = await openPage();
    const hostComputer = member(page, 'shell', 'host_computer');

    expect(await hostComputer.isVisible()).toBe(true);
    await page.locator('.tag-filter button', { hasText: /^method$/ }).click();

    await expect.poll(() => hostComputer.isVisible()).toBe(false);
  });

  it('should hide a type whose members all have an excluded tag', async () => {
    const page = await openPage();
    const admin = section(page, 'admin');
    const contentsEntry = page.locator('#panel-api nav.contents > ul > li', { hasText: /^adminComputer/ });

    expect(await admin.isVisible()).toBe(true);
    await page.locator('.tag-filter button', { hasText: /^greybel$/ }).click();

    await expect.poll(() => admin.isVisible()).toBe(false);
    expect(await section(page, 'adminNetwork').isVisible()).toBe(false);
    expect(await contentsEntry.isVisible()).toBe(false);
    expect(await section(page, 'shell').isVisible()).toBe(true);
  });

  it('should scroll to the member a description links to', async () => {
    const page = await openPage();
    const getShell = member(page, 'general', 'get_shell');

    await section(page, 'shell')
      .locator('.description a', { hasText: /^get_shell$/ })
      .first()
      .click();

    await expect.poll(async () => (await getShell.boundingBox())?.y ?? -1).toBeGreaterThanOrEqual(0);
    expect((await getShell.boundingBox())!.y).toBeLessThan(200);
  });

  it('should clear the search for a link to what it hides', async () => {
    const page = await openPage();
    const computer = section(page, 'computer');

    await page.locator('input[aria-label="Search the API"]').fill('shell.host');
    await expect.poll(() => computer.isVisible()).toBe(false);
    await member(page, 'shell', 'host_computer')
      .locator('h3 a', { hasText: /^computer$/ })
      .click();

    await expect.poll(async () => (await computer.boundingBox())?.y ?? -1).toBeGreaterThanOrEqual(0);
    expect((await computer.boundingBox())!.y).toBeLessThan(200);
    expect(await page.locator('input[aria-label="Search the API"]').inputValue()).toBe('');
  });

  it('should highlight examples and link their general functions', async () => {
    const page = await openPage();
    const example = member(page, 'shell', 'host_computer').locator('pre.example');

    expect(await example.textContent()).toBe(
      'shell = get_shell\ncomputer = shell.host_computer\nprint("Computer public IP is: " + computer.public_ip)'
    );
    expect(await example.locator('.token.string').first().textContent()).toBe('"Computer public IP is: "');
    expect(await example.locator('a', { hasText: 'get_shell' }).getAttribute('href')).toBe('#api-general.get_shell');
  });

  it('should request the editor to open an example', async () => {
    const page = await openPage();
    const hostComputer = member(page, 'shell', 'host_computer');

    await hostComputer.locator('button', { hasText: 'Open in editor' }).click();

    expect(await page.evaluate(() => referencePage.openedDocument())).toEqual({
      content: await hostComputer.locator('pre.example').textContent()
    });
  });

  it('should show the API first, with its search, and the guide in its own tab without it', async () => {
    const page = await openPage();

    expect(await page.locator('.overview h1').textContent()).toBe('GreyScript API – Unofficial Documentation');
    expect(await page.locator('.tab[aria-selected="true"]').textContent()).toBe('API reference');
    expect(await section(page, 'shell').isVisible()).toBe(true);
    expect(await chapter(page, 'Values').isVisible()).toBe(false);

    await openGuide(page);

    expect(await chapter(page, 'Values').isVisible()).toBe(true);
    expect(await section(page, 'shell').isVisible()).toBe(false);
    expect(await page.locator('input[aria-label="Search the API"]').count()).toBe(0);
    expect(await page.locator('input[aria-label="Search the guide"]').count()).toBe(1);
    expect(await page.locator('.introduction a', { hasText: /^MiniScript$/ }).getAttribute('href')).toBe(
      'https://miniscript.org/'
    );
  });

  it('should list each chapter with its parts in the sidebar of the guide', async () => {
    const page = await openPage();

    await openGuide(page);

    const chapters = page.locator('#panel-guide nav.contents > ul > li');

    await expect.poll(() => chapters.count()).toBe(13);
    expect(await chapters.first().locator('a.entry').textContent()).toBe('Syntax');
    expect(await chapters.nth(1).locator('ul a').allTextContents()).toEqual([
      'Numbers',
      'Strings',
      'Lists',
      'Maps',
      'Indexing',
      'null',
      'Functions'
    ]);

    expect(await chapters.nth(10).locator('a.entry').textContent()).toBe('Syntax features');
    expect(await chapters.nth(10).locator('ul a').allTextContents()).toContain('Classes');

    const lists = page.locator('#guide-values-lists');

    await chapters
      .nth(1)
      .locator('ul a', { hasText: /^Lists$/ })
      .click();

    await expect.poll(async () => (await lists.boundingBox())?.y ?? -1).toBeGreaterThanOrEqual(0);
    expect((await lists.boundingBox())!.y).toBeLessThan(200);
  });

  it('should highlight the code of the guide as the examples, without indenting its first line', async () => {
    const page = await openPage();
    const values = chapter(page, 'Values');

    await openGuide(page);

    expect(await values.locator('h3').first().textContent()).toBe('Numbers');
    expect(await values.locator('pre.example .token.number').first().textContent()).toBe('7');
    expect(await values.locator('button', { hasText: 'Open in editor' }).count()).toBeGreaterThan(0);

    const first = values.locator('pre.example').first();
    const indentation = await first.evaluate((pre) => {
      const code = pre.querySelector('code')!;

      return [getComputedStyle(code).paddingLeft, getComputedStyle(code).backgroundColor];
    });

    expect(indentation).toEqual(['0px', 'rgba(0, 0, 0, 0)']);
  });

  it('should space the tags of the filter apart', async () => {
    const page = await openPage();
    const tags = page.locator('.tag-filter .tag');
    const first = (await tags.nth(0).boundingBox())!;
    const second = (await tags.nth(1).boundingBox())!;

    expect(second.x - (first.x + first.width)).toBeGreaterThan(2);
  });

  it('should show code of another language as plain text', async () => {
    const page = await openPage();
    const json = chapter(page, 'Tools').locator('pre.example[data-language="json"]').first();

    await openGuide(page);
    await json.waitFor();
    expect(await json.locator('.token').count()).toBe(0);
    expect(await json.locator('xpath=..').locator('button', { hasText: 'Open in editor' }).count()).toBe(0);
  });

  it('should scroll to the chapter a chapter links to', async () => {
    const page = await openPage();
    const classes = chapter(page, 'Classes and objects');

    await openGuide(page);
    await chapter(page, 'Operators')
      .locator('.description a', { hasText: /^Classes$/ })
      .click();

    await expect.poll(async () => (await classes.boundingBox())?.y ?? -1).toBeGreaterThanOrEqual(0);
    expect((await classes.boundingBox())!.y).toBeLessThan(200);
  });

  it('should open the tab a link leads to', async () => {
    const page = await openPage();
    const range = member(page, 'general', 'range');
    const values = chapter(page, 'Values');

    await openGuide(page);
    await chapter(page, 'Control flow')
      .locator('.description a', { hasText: /^range$/ })
      .first()
      .click();

    await expect.poll(() => range.isVisible()).toBe(true);
    expect(await page.locator('.tab[aria-selected="true"]').textContent()).toBe('API reference');
    expect((await range.boundingBox())!.y).toBeLessThan(200);

    // No description of the API links to the guide yet: the guide's sidebar, hidden with its tab, has such links.
    await page.locator('#panel-guide nav.contents a.entry', { hasText: /^Values$/ }).evaluate((link) => {
      (link as HTMLAnchorElement).click();
    });

    await expect.poll(() => values.isVisible()).toBe(true);
    expect(await page.locator('.tab[aria-selected="true"]').textContent()).toBe('Guide');
  });

  it('should keep where each tab was scrolled to', async () => {
    const page = await openPage();

    await member(page, 'shell', 'host_computer').scrollIntoViewIfNeeded();

    const position = await page.evaluate(() => window.scrollY);

    await openGuide(page);
    await page.locator('.tab', { hasText: 'API reference' }).click();

    expect(await page.evaluate(() => window.scrollY)).toBe(position);
  });

  it('should link every part of the guide from its heading', async () => {
    const page = await openPage();
    const heading = page.locator('#guide-values-lists');

    await openGuide(page);
    await chapter(page, 'Syntax').scrollIntoViewIfNeeded();
    await heading.hover();

    expect(await heading.locator('a.heading-link').getAttribute('href')).toBe('#guide-values-lists');
    expect(await page.locator('#guide-values a.heading-link').getAttribute('href')).toBe('#guide-values');

    await heading.locator('a.heading-link').click();

    await expect.poll(async () => (await heading.boundingBox())?.y ?? -1).toBeGreaterThanOrEqual(0);
    expect((await heading.boundingBox())!.y).toBeLessThan(200);
  });

  it('should show the tables of the guide as tables', async () => {
    const page = await openPage();
    const table = chapter(page, 'Tools').locator('table').first();

    await openGuide(page);

    expect(await table.locator('thead th').count()).toBeGreaterThan(1);
    expect(await table.locator('tbody tr').count()).toBeGreaterThan(2);
  });

  it('should search the guide by its parts', async () => {
    const page = await openPage();
    const logic = page.locator('#guide-operators-logic');

    await openGuide(page);
    await page.locator('#panel-guide').waitFor();
    await page.locator('input[aria-label="Search the guide"]').fill('short-circuit evaluation');

    await expect.poll(() => chapter(page, 'Values').isVisible()).toBe(false);
    expect(await logic.isVisible()).toBe(true);
    expect(await page.locator('#guide-operators-arithmetic').isVisible()).toBe(false);
    expect(await page.locator('#panel-guide nav.contents a', { hasText: /^Logic$/ }).isVisible()).toBe(true);
    expect(await page.locator('#panel-guide nav.contents a', { hasText: /^Arithmetic$/ }).isVisible()).toBe(false);

    await page.locator('input[aria-label="Search the guide"]').fill('no such words anywhere');

    await expect.poll(() => page.locator('#panel-guide .no-results').isVisible()).toBe(true);
  });

  it('should show the button that clears a search whenever it has text', async () => {
    const page = await openPage();
    const search = page.locator('input[aria-label="Search the API"]');
    const clear = page.locator('#panel-api, .toolbar').locator('button.clear');

    expect(await clear.count()).toBe(0);
    await search.fill('shell.host');
    await page.locator('.overview h1').click();

    expect(await clear.isVisible()).toBe(true);
    await clear.click();

    expect(await search.inputValue()).toBe('');
    expect(await clear.count()).toBe(0);
    await expect.poll(() => section(page, 'general').isVisible()).toBe(true);
  });
});
