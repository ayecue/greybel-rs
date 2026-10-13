import { describe, expect, it } from 'vitest';

import { clickBreakpointGutter, openEditor, terminalText } from './support/browser.js';

describe('the editor', () => {
  it('should highlight the code with the language server and show its problems', async () => {
    const page = await openEditor('f = function(a)\n  return a + 1\nend function\nprint(f(1)\nif x then');

    await expect.poll(() => page.locator('.cm-token-keyword').count()).toBeGreaterThan(0);
    expect(await page.locator('.cm-token-parameter').first().textContent()).toBe('a');
    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBeGreaterThan(0);
  });

  it('should run a program and show its output', async () => {
    const page = await openEditor('print("<b>hello</b> " + params[0])');

    await page.locator('.params').fill('world');
    await page.locator('button.run').click();

    await expect.poll(() => terminalText(page)).toContain('hello</b> world');
    await expect.poll(() => page.locator('button.run').isEnabled()).toBe(true);
  });

  it('should wait between clearing the screen and printing again', async () => {
    const page = await openEditor(
      'for i in range(3)\n\twait(0.2)\n\tclear_screen\n\tprint(("#" * (3 - i)) + ("-" * i))\nend for'
    );

    const start = Date.now();

    await page.locator('button.run').click();
    await expect.poll(() => page.locator('button.run').isEnabled(), { timeout: 10_000 }).toBe(true);

    expect(Date.now() - start).toBeGreaterThanOrEqual(800);
    expect((await terminalText(page)).trim()).toBe('###');
  });

  it('should read the input typed on the terminal', async () => {
    const page = await openEditor('name = user_input("Name: ")\nprint("hi " + name)');

    await page.locator('button.run').click();
    await expect.poll(() => terminalText(page)).toContain('Name: ');
    await page.keyboard.type('Ada');
    await page.keyboard.press('Enter');

    await expect.poll(() => terminalText(page)).toContain('hi Ada');
  });

  it('should stop at a breakpoint and run code where it stopped', async () => {
    const page = await openEditor('x = 21\ny = x * 2\nprint(y)');

    await clickBreakpointGutter(page, 2);
    await page.locator('button.debug').click();
    await page.locator('.debug-panel').waitFor();

    expect(await page.locator('.cm-stop-line').textContent()).toBe('y = x * 2');
    await page.locator('.debug-evaluate').fill('x + 1');
    await page.locator('.debug-evaluate').press('Enter');
    await expect.poll(() => page.locator('.debug-log').textContent()).toContain('22');
    await page.locator('button.debug-continue').click();

    await expect.poll(() => terminalText(page)).toContain('42');
  });

  it('should transform the code', async () => {
    const page = await openEditor('value = 1 // a comment\nprint(value)');

    await page.locator('.transform-mode').selectOption('uglify');
    await page.locator('button.transform').click();

    await expect.poll(() => page.locator('.transform-result').inputValue()).toMatch(/\bprint\([A-Za-z]\)$/);
    expect(await page.locator('.transform-result').inputValue()).not.toMatch(/comment|value/);
  });

  it('should share the code by a link', async () => {
    const page = await openEditor('print(1)');

    await page.locator('button.share').click();

    const link = new URL(await page.locator('.link-popup textarea').inputValue());

    expect(decodeURIComponent(atob(link.searchParams.get('c') ?? ''))).toBe('print(1)');
  });

  it('should insert the examples of the API reference into the code', async () => {
    const page = await openEditor('');

    await page.locator('button.reference-toggle').click();
    await page.locator('.reference-panel article.member').first().waitFor();

    const shell = page.locator('.reference-panel section.section', {
      has: page.locator('h2', { hasText: /^shell( |$)/ })
    });
    const hostComputer = shell.locator('article.member', {
      has: page.locator('h3 .name', { hasText: /^host_computer$/ })
    });

    await hostComputer.locator('button', { hasText: 'Open in editor' }).click();

    await expect.poll(() => page.locator('.cm-content').textContent()).toContain('host_computer');
  });
});
