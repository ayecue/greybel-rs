import { describe, expect, it } from 'vitest';

import {
  clickBreakpointGutter,
  openEditor,
  openFile,
  projectText,
  shownFile,
  tab,
  tabPaths,
  terminalText
} from './support/browser.js';
import { FILES, UTIL } from './support/project.js';

describe('the tabs of the files opened', () => {
  it('should open a tab per file, after the selected one', async () => {
    const page = await openEditor(projectText('main.src', FILES));

    expect(await tabPaths(page)).toEqual(['main.src']);
    await openFile(page, 'b.src');
    await tab(page, 'main.src').click();
    await openFile(page, 'a.src');

    expect(await tabPaths(page)).toEqual(['main.src', 'a.src', 'b.src']);
    expect(await shownFile(page)).toBe('a.src');
    await tab(page, 'b.src').click();
    expect(await page.locator('.cm-content').textContent()).toBe('print("b")');
  });

  it('should show the tab beside the one closed, and no file once all are closed', async () => {
    const page = await openEditor(projectText('main.src', FILES));

    await openFile(page, 'a.src');
    await openFile(page, 'b.src');
    await tab(page, 'a.src').click();
    await tab(page, 'a.src').locator('.close-tab').click();

    expect(await tabPaths(page)).toEqual(['main.src', 'b.src']);
    expect(await shownFile(page)).toBe('b.src');
    await tab(page, 'b.src').click({ button: 'middle' });
    await tab(page, 'main.src').locator('.close-tab').click();

    expect(await tabPaths(page)).toEqual([]);
    expect(await shownFile(page)).toBeNull();
    expect(await page.locator('.no-file').isVisible()).toBe(true);
  });

  it('should retain the edits and breakpoints of a file whose tab was closed', async () => {
    const page = await openEditor(
      projectText('main.src', { 'main.src': 'import_code("/util.src")\nprint(greet("Ada"))', 'util.src': UTIL })
    );

    await openFile(page, 'util.src');
    await clickBreakpointGutter(page, 2);
    await page.locator('.cm-content').click();
    await page.keyboard.press('ControlOrMeta+End');
    await page.keyboard.insertText('\n// edited');
    await tab(page, 'util.src').locator('.close-tab').click();
    await openFile(page, 'util.src');

    expect(await page.locator('.cm-content').textContent()).toContain('// edited');
    expect(await page.locator('.cm-breakpoint:visible').count()).toBe(1);
    await tab(page, 'util.src').locator('.close-tab').click();
    await page.locator('button.debug').click();
    await page.locator('.debug-panel').waitFor();

    expect(await tabPaths(page)).toEqual(['main.src', 'util.src']);
    expect(await shownFile(page)).toBe('util.src');
    await page.locator('button.debug-continue').click();
    await expect.poll(() => terminalText(page)).toContain('hi Ada');
  });

  it('should rename and close the tab of a renamed and deleted file', async () => {
    const page = await openEditor(projectText('main.src', FILES));

    await openFile(page, 'a.src');
    await page.locator('.file-list li[data-path="a.src"] .file-path').dblclick();
    await page.locator('input.file-name').fill('lib/c.src');
    await page.locator('input.file-name').press('Enter');

    await expect.poll(() => tabPaths(page)).toEqual(['main.src', 'lib/c.src']);
    expect(await tab(page, 'lib/c.src').locator('.tab-name').textContent()).toBe('c.src');
    await page.locator('.file-list li[data-path="lib/c.src"] .delete-file').click();

    await expect.poll(() => tabPaths(page)).toEqual(['main.src']);
    expect(await shownFile(page)).toBe('main.src');
  });
});
