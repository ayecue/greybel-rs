import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';
import { WORLD_PROJECT } from './support/project.js';

describe('the world of the page', () => {
  it('should keep the world from one run to the next until it is reset', async () => {
    const editor = await EditorPage.open(WORLD_PROJECT);
    const { page } = editor;

    await editor.runToEnd('write');
    await editor.runToEnd('');
    expect(await editor.getTerminalText()).toContain('content: from the writer');

    await page.locator('button.clear').click();
    await page.locator('button.reset-world').click();
    await editor.runToEnd('');
    expect(await editor.getTerminalText()).toContain('content: none');
  });

  it("should let a terminal's program read what another terminal's program wrote while both run", async () => {
    const editor = await EditorPage.open(WORLD_PROJECT);
    const { page } = editor;

    await editor.selectProfile('wait');
    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('Read? ');

    await page.locator('button.terminal-add').click();
    await expect.poll(() => editor.getTerminalTab('Terminal 2').getAttribute('aria-selected')).toBe('true');
    await editor.runToEnd('write');
    expect(await editor.getTerminalText()).toContain('written');

    await editor.getTerminalTab('Terminal 1').click();
    expect(await editor.getTerminalTab('Terminal 1').getAttribute('data-state')).toBe('running');
    await page.locator('.terminals canvas:not([hidden])').click();
    await page.keyboard.press('Enter');

    await expect.poll(() => editor.getTerminalText()).toContain('content: from the writer');
  });

  it('should close a terminal, stopping its program', async () => {
    const editor = await EditorPage.open(WORLD_PROJECT);
    const { page } = editor;

    await page.locator('button.terminal-add').click();
    await editor.selectProfile('wait');
    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('Read? ');

    await editor.getTerminalTab('Terminal 2').locator('.terminal-close').click();

    expect(await page.locator('.terminal-tab').count()).toBe(1);
    expect(await page.locator('.terminal-close').count()).toBe(0);
    expect(await page.locator('button.run').isEnabled()).toBe(true);
  });
});
