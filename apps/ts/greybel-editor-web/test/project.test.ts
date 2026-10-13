import { describe, expect, it } from 'vitest';

import {
  clickBreakpointGutter,
  fileEntry,
  openEditor,
  projectText,
  shownFile,
  terminalText,
  typeCode
} from './support/browser.js';
import { UTIL } from './support/project.js';

describe('a project of several files', () => {
  it('should run the entry file with the files it imports', async () => {
    const page = await openEditor(
      projectText('main.src', {
        'main.src': 'import_code("/lib/util.src")\n#include "helpers.src"\nprint(greet(who))',
        'lib/util.src': UTIL,
        'helpers.src': 'who = "Ada"'
      })
    );

    expect(await page.locator('.file-list li.file').count()).toBe(3);
    await page.locator('button.run').click();

    await expect.poll(() => terminalText(page)).toContain('hi Ada');
  });

  it('should show the problems of an import only while its file is missing', async () => {
    const page = await openEditor(projectText('main.src', { 'main.src': '#include "util.src"\nprint(greet("x"))' }));

    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBeGreaterThan(0);
    await page.locator('button.new-file').click();
    await page.locator('input.file-name').fill('util.src');
    await page.locator('input.file-name').press('Enter');
    await expect.poll(() => shownFile(page)).toBe('util.src');
    await typeCode(page, UTIL);
    await fileEntry(page, 'main.src').locator('.file-path').click();

    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBe(0);
  });

  it('should stop at a breakpoint of an imported file and show it', async () => {
    const page = await openEditor(
      projectText('main.src', { 'main.src': 'import_code("/util.src")\nprint(greet("Ada"))', 'util.src': UTIL })
    );

    await fileEntry(page, 'util.src').locator('.file-path').click();
    await clickBreakpointGutter(page, 2);
    await fileEntry(page, 'main.src').locator('.file-path').click();
    await page.locator('button.debug').click();
    await page.locator('.debug-panel').waitFor();

    expect(await shownFile(page)).toBe('util.src');
    expect(await page.locator('.cm-stop-line').textContent()).toBe('  return "hi " + name');
    await page.locator('button.debug-continue').click();

    await expect.poll(() => terminalText(page)).toContain('hi Ada');
  });

  it('should run from the file marked as the entry', async () => {
    const page = await openEditor(
      projectText('main.src', { 'main.src': 'print("main")', 'other.src': 'print("other")' })
    );

    await fileEntry(page, 'other.src').locator('.entry-marker').click();
    await page.locator('button.run').click();

    await expect.poll(() => terminalText(page)).toContain('other');
    expect(await terminalText(page)).not.toContain('main');
  });

  it('should rename and delete files', async () => {
    const page = await openEditor(projectText('main.src', { 'main.src': 'print(1)', 'old.src': 'print(2)' }));

    await fileEntry(page, 'old.src').locator('.file-path').dblclick();
    await page.locator('input.file-name').fill('lib/new.src');
    await page.locator('input.file-name').press('Enter');
    await expect.poll(() => shownFile(page)).toBe('lib/new.src');
    expect(await page.locator('.cm-content').textContent()).toBe('print(2)');
    await fileEntry(page, 'main.src').locator('.delete-file').click();

    await expect.poll(() => page.locator('.file-list li.file').count()).toBe(1);
    expect(await fileEntry(page, 'lib/new.src').locator('.entry-marker').getAttribute('aria-pressed')).toBe('true');
  });

  it('should share every file by a link', async () => {
    const page = await openEditor(projectText('main.src', { 'main.src': 'print(1)', 'util.src': UTIL }));

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
