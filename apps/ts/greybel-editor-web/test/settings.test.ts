import type { Page } from 'playwright';
import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';

describe("the project's greybel.json", () => {
  it('should be listed with its template until its first edit creates it', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;
    const entry = editor.getFileEntry('greybel.json');

    expect(await entry.getAttribute('class')).toContain('template');
    await editor.openFile('greybel.json');
    expect(await editor.getShownFile()).toBe('greybel.json');
    expect(await page.locator('.cm-content').textContent()).toContain('"mode": "Uglify"');
    await editor.typeCode('{ "build": { "mode": "Beautify" } }');

    await expect.poll(() => entry.getAttribute('class')).not.toContain('template');
    await expect.poll(() => page.locator('.transform-mode').textContent()).toBe('Beautify');
  });

  it("should show the language server's problem with it on its line and keep the last valid settings", async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', {
        'main.src': 'print(1)',
        'greybel.json': '{ "build": { "mode": "Default" } }'
      })
    );
    const { page } = editor;

    await expect.poll(() => page.locator('.transform-mode').textContent()).toBe('Default');
    await editor.openFile('greybel.json');
    await editor.typeCode('{\n  "build": { "mode": "Default" },\n  "unknown": 1\n}');

    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBe(1);
    expect(await page.locator('.cm-lintRange-error').textContent()).toBe('"unknown": 1');
    expect(await page.locator('.transform-mode').textContent()).toBe('Default');
  });

  it('should give the language server its strictMode', async () => {
    const editor = await EditorPage.open(EditorPage.encodeProject('main.src', { 'main.src': 'x = 1\nx += 1' }));
    const { page } = editor;

    await expect.poll(() => page.locator('.cm-token-variable').count()).toBeGreaterThan(0);
    expect(await page.locator('.cm-lintRange-error').count()).toBe(0);
    await editor.openFile('greybel.json');
    await editor.typeCode('{ "strictMode": true }');
    await editor.openFile('main.src');

    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBeGreaterThan(0);
  });

  it('should complete and describe its settings', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    await editor.openFile('greybel.json');
    await editor.typeCode('{ "run": { "');
    await page.keyboard.press('Control+Space');
    await page.locator('.cm-tooltip-autocomplete').waitFor();
    expect(await page.locator('.cm-tooltip-autocomplete').textContent()).toContain('seed');

    await page.keyboard.press('Escape');
    await editor.typeCode('{ "run": { "seed": "x" } }');
    await editor.hoverCode('"seed"');

    await expect.poll(() => page.locator('.cm-manifest-hover').textContent()).toContain('Seed text');
  });
});

describe('the world', () => {
  it('should be generated from run.seed, also by New world', async () => {
    const seeds = (page: Page) => page.workers().flatMap((worker) => new URL(worker.url()).searchParams.getAll('seed'));
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', {
        'main.src': 'print(1)',
        'greybel.json': JSON.stringify({ run: { seed: 'other' } })
      })
    );
    const { page } = editor;

    await expect.poll(() => seeds(page)).toContain('other');
    await editor.openFile('greybel.json');
    await editor.typeCode(JSON.stringify({ run: { seed: 'third' } }));
    await page.locator('button.reset-world').click();

    await expect.poll(() => seeds(page)).toContain('third');
  });
});

describe('the terminals', () => {
  it('should each run their own script', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('a.src', { 'a.src': 'print("ran a")', 'b.src': 'print("ran b")' })
    );
    const { page } = editor;

    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('ran a');
    await page.locator('button.terminal-add').click();
    await page.locator('select.script').selectOption('b.src');
    await page.locator('button.run').click();

    await expect.poll(() => editor.getTerminalText()).toContain('ran b');
    expect(await editor.getTerminalText()).not.toContain('ran a');
    expect(await page.locator('.terminal-tab.active .terminal-script').textContent()).toBe('b.src');
  });

  it('should split long lines by run.lineLimits', async () => {
    const script = [
      'line = ""',
      'for i in range(1, 250)',
      '  line = line + "<color=#ff0000>x</color>"',
      'end for',
      'print(line)',
      'print("done")'
    ].join('\n');
    const lineCount = async (manifest: object) => {
      const editor = await EditorPage.open(
        EditorPage.encodeProject('main.src', { 'main.src': script, 'greybel.json': JSON.stringify(manifest) })
      );
      const { page } = editor;

      await page.locator('button.run').click();
      await expect.poll(() => editor.getTerminalText()).toContain('done');
      await page.locator('button.copy-terminal').click();

      const readClipboard = () => page.evaluate(() => navigator.clipboard.readText());

      await expect.poll(readClipboard).toContain('done');

      const text = await readClipboard();

      return text.split('\n').filter((line) => line !== '' && line !== 'done').length;
    };

    expect(await lineCount({})).toBe(34);
    expect(await lineCount({ run: { lineLimits: { textPerLine: 256 } } })).toBe(1);
  });

  it('should follow the renaming of their script', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('a.src', { 'a.src': 'print("ran a")', 'b.src': 'print(1)' })
    );
    const { page } = editor;

    await page.locator('select.script').selectOption('a.src');
    await editor.getFileEntry('a.src').locator('.file-path').dblclick();
    await page.locator('input.file-name').fill('c.src');
    await page.locator('input.file-name').press('Enter');

    await expect.poll(() => page.locator('.terminal-tab.active .terminal-script').textContent()).toBe('c.src');
    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('ran a');
  });
});
