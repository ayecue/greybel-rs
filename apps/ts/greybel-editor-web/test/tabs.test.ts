import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';
import { FILES, UTIL } from './support/project.js';

describe('the tabs of the files opened', () => {
  it('should open a tab per file, after the selected one', async () => {
    const editor = await EditorPage.open(EditorPage.encodeProject('main.src', FILES));
    const { page } = editor;

    expect(await editor.getTabPaths()).toEqual(['main.src']);
    await editor.openFile('b.src');
    await editor.getTab('main.src').click();
    await editor.openFile('a.src');

    expect(await editor.getTabPaths()).toEqual(['main.src', 'a.src', 'b.src']);
    expect(await editor.getShownFile()).toBe('a.src');
    await editor.getTab('b.src').click();
    expect(await page.locator('.cm-content').textContent()).toBe('print("b")');
  });

  it('should show the tab beside the one closed, and no file once all are closed', async () => {
    const editor = await EditorPage.open(EditorPage.encodeProject('main.src', FILES));
    const { page } = editor;

    await editor.openFile('a.src');
    await editor.openFile('b.src');
    await editor.getTab('a.src').click();
    await editor.getTab('a.src').locator('.close-tab').click();

    expect(await editor.getTabPaths()).toEqual(['main.src', 'b.src']);
    expect(await editor.getShownFile()).toBe('b.src');
    await editor.getTab('b.src').click({ button: 'middle' });
    await editor.getTab('main.src').locator('.close-tab').click();

    expect(await editor.getTabPaths()).toEqual([]);
    expect(await editor.getShownFile()).toBeNull();
    expect(await page.locator('.no-file').isVisible()).toBe(true);
  });

  it('should retain the edits and breakpoints of a file whose tab was closed', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', {
        'main.src': 'import_code("/util.src")\nprint(greet("Ada"))',
        'util.src': UTIL
      })
    );
    const { page } = editor;

    await editor.openFile('util.src');
    await editor.clickBreakpointGutter(2);
    await page.locator('.cm-content').click();
    await page.keyboard.press('ControlOrMeta+End');
    await page.keyboard.insertText('\n// edited');
    await editor.getTab('util.src').locator('.close-tab').click();
    await editor.openFile('util.src');

    expect(await page.locator('.cm-content').textContent()).toContain('// edited');
    expect(await page.locator('.cm-breakpoint:visible').count()).toBe(1);
    await editor.getTab('util.src').locator('.close-tab').click();
    await page.locator('button.debug').click();
    await editor.waitForStop();

    expect(await editor.getTabPaths()).toEqual(['main.src', 'util.src']);
    expect(await editor.getShownFile()).toBe('util.src');
    await page.locator('button.debug-continue').click();
    await expect.poll(() => editor.getTerminalText()).toContain('hi Ada');
  });

  it('should rename and close the tab of a renamed and deleted file', async () => {
    const editor = await EditorPage.open(EditorPage.encodeProject('main.src', FILES));
    const { page } = editor;

    await editor.openFile('a.src');
    await page.locator('.file-list li[data-path="a.src"] .file-path').dblclick();
    await page.locator('input.file-name').fill('lib/c.src');
    await page.locator('input.file-name').press('Enter');

    await expect.poll(() => editor.getTabPaths()).toEqual(['main.src', 'lib/c.src']);
    expect(await editor.getTab('lib/c.src').locator('.tab-name').textContent()).toBe('c.src');
    await page.locator('.file-list li[data-path="lib/c.src"] .delete-file').click();

    await expect.poll(() => editor.getTabPaths()).toEqual(['main.src']);
    expect(await editor.getShownFile()).toBe('main.src');
  });
});
