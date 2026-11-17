import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';
import { UTIL } from './support/project.js';

describe('a project of several files', () => {
  it('should run the entry file with the files it imports', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', {
        'main.src': 'import_code("/lib/util.src")\n#include "helpers.src"\nprint(greet(who))',
        'lib/util.src': UTIL,
        'helpers.src': 'who = "Ada"'
      })
    );
    const { page } = editor;

    expect(await page.locator('.file-list li.file').count()).toBe(3);
    await page.locator('button.run').click();

    await expect.poll(() => editor.getTerminalText()).toContain('hi Ada');
  });

  it('should show the problems of an import only while its file is missing', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', { 'main.src': '#include "util.src"\nprint(greet("x"))' })
    );
    const { page } = editor;

    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBeGreaterThan(0);
    await page.locator('button.new-file').click();
    await page.locator('input.file-name').fill('util.src');
    await page.locator('input.file-name').press('Enter');
    await expect.poll(() => editor.getShownFile()).toBe('util.src');
    await editor.typeCode(UTIL);
    await editor.getFileEntry('main.src').locator('.file-path').click();

    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBe(0);
  });

  it('should stop at a breakpoint of an imported file and show it', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', {
        'main.src': 'import_code("/util.src")\nprint(greet("Ada"))',
        'util.src': UTIL
      })
    );
    const { page } = editor;

    await editor.getFileEntry('util.src').locator('.file-path').click();
    await editor.clickBreakpointGutter(2);
    await editor.getFileEntry('main.src').locator('.file-path').click();
    await page.locator('button.debug').click();
    await page.locator('.debug-panel').waitFor();

    expect(await editor.getShownFile()).toBe('util.src');
    expect(await page.locator('.cm-stop-line').textContent()).toBe('  return "hi " + name');
    await page.locator('button.debug-continue').click();

    await expect.poll(() => editor.getTerminalText()).toContain('hi Ada');
  });

  it('should run from the file marked as the entry', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', { 'main.src': 'print("main")', 'other.src': 'print("other")' })
    );
    const { page } = editor;

    await editor.getFileEntry('other.src').locator('.entry-marker').click();
    await page.locator('button.run').click();

    await expect.poll(() => editor.getTerminalText()).toContain('other');
    expect(await editor.getTerminalText()).not.toContain('main');
  });

  it('should rename and delete files', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', { 'main.src': 'print(1)', 'old.src': 'print(2)' })
    );
    const { page } = editor;

    await editor.getFileEntry('old.src').locator('.file-path').dblclick();
    await page.locator('input.file-name').fill('lib/new.src');
    await page.locator('input.file-name').press('Enter');
    await expect.poll(() => editor.getShownFile()).toBe('lib/new.src');
    expect(await page.locator('.cm-content').textContent()).toBe('print(2)');
    await editor.getFileEntry('main.src').locator('.delete-file').click();

    await expect.poll(() => page.locator('.file-list li.file').count()).toBe(1);
    expect(await editor.getFileEntry('lib/new.src').locator('.entry-marker').getAttribute('aria-pressed')).toBe('true');
  });

  it('should share every file by a link', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', { 'main.src': 'print(1)', 'util.src': UTIL })
    );
    const { page } = editor;

    await page.locator('button.share').click();

    const link = new URL(await page.locator('.link-popup textarea').inputValue());
    const shared = JSON.parse(decodeURIComponent(atob(link.searchParams.get('c') ?? ''))) as unknown;

    expect(shared).toEqual({
      greybel: 'project',
      version: 1,
      entry: 'main.src',
      files: { 'main.src': 'print(1)', 'util.src': UTIL }
    });
  });
});
