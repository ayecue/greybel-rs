import { describe, expect, it } from 'vitest';

import { openEditor, terminalText } from './support/browser.js';
import { PROGRAM, runToEnd, terminalTab } from './support/world.js';

describe('the world of the page', () => {
  it('should keep the world from one run to the next until it is reset', async () => {
    const page = await openEditor(PROGRAM);

    await runToEnd(page, 'write');
    await runToEnd(page, '');
    expect(await terminalText(page)).toContain('content: from the writer');

    await page.locator('button.clear').click();
    await page.locator('button.reset-world').click();
    await runToEnd(page, '');
    expect(await terminalText(page)).toContain('content: none');
  });

  it("should let a terminal's program read what another terminal's program wrote while both run", async () => {
    const page = await openEditor(PROGRAM);

    await page.locator('.params').fill('wait');
    await page.locator('button.run').click();
    await expect.poll(() => terminalText(page)).toContain('Read? ');

    await page.locator('button.terminal-add').click();
    await expect.poll(() => terminalTab(page, 'Terminal 2').getAttribute('aria-selected')).toBe('true');
    await runToEnd(page, 'write');
    expect(await terminalText(page)).toContain('written');

    await terminalTab(page, 'Terminal 1').click();
    expect(await terminalTab(page, 'Terminal 1').getAttribute('data-state')).toBe('running');
    await page.locator('.terminals canvas:not([hidden])').click();
    await page.keyboard.press('Enter');

    await expect.poll(() => terminalText(page)).toContain('content: from the writer');
  });

  it('should close a terminal, stopping its program', async () => {
    const page = await openEditor(PROGRAM);

    await page.locator('button.terminal-add').click();
    await page.locator('.params').fill('wait');
    await page.locator('button.run').click();
    await expect.poll(() => terminalText(page)).toContain('Read? ');

    await terminalTab(page, 'Terminal 2').locator('.terminal-close').click();

    expect(await page.locator('.terminal-tab').count()).toBe(1);
    expect(await page.locator('.terminal-close').count()).toBe(0);
    expect(await page.locator('button.run').isEnabled()).toBe(true);
  });
});
