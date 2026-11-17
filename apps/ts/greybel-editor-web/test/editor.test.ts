import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';

describe('the editor', () => {
  it('should highlight the code with the language server and show its problems', async () => {
    const editor = await EditorPage.open('f = function(a)\n  return a + 1\nend function\nprint(f(1)\nif x then');
    const { page } = editor;

    await expect.poll(() => page.locator('.cm-token-keyword').count()).toBeGreaterThan(0);
    expect(await page.locator('.cm-token-parameter').first().textContent()).toBe('a');
    await expect.poll(() => page.locator('.cm-lintRange-error').count()).toBeGreaterThan(0);
  });

  it('should colour types and the keywords of syntax features', async () => {
    const editor = await EditorPage.open('let port: number = 22\nf = function(host: string): map|null\nend function');
    const { page } = editor;
    const types = page.locator('.cm-token-type');

    await expect.poll(() => types.allTextContents()).toEqual(['number', 'string', 'map']);
    expect(await types.first().evaluate((element) => getComputedStyle(element).color)).toBe('rgb(78, 201, 176)');
    expect(await page.locator('.cm-token-keyword').first().textContent()).toBe('let');
  });

  it('should highlight the signature a hover shows', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    await expect.poll(() => page.locator('.cm-token-function').count()).toBeGreaterThan(0);
    await editor.hoverCode('print');

    const name = page.locator('.cm-lsp-hover-tooltip code.language-greyscript-signature span', { hasText: /^print$/ });

    await name.waitFor();
    expect(await name.evaluate((element) => getComputedStyle(element).color)).toBe('rgb(220, 220, 170)');
    expect(
      await page.locator('.cm-lsp-hover-tooltip code.language-greyscript-signature span', { hasText: /^null$/ }).count()
    ).toBe(1);
  });

  it('should run a program and show its output', async () => {
    const editor = await EditorPage.open(
      EditorPage.encodeProject('main.src', {
        'main.src': 'print("<b>hello</b> " + params[0])',
        'greybel.json': JSON.stringify({ run: { params: ['world'] } })
      })
    );
    const { page } = editor;

    await page.locator('button.run').click();

    await expect.poll(() => editor.getTerminalText()).toContain('hello</b> world');
    await expect.poll(() => page.locator('button.run').isEnabled()).toBe(true);
  });

  it("should copy the terminal's text without its rich-text tags", async () => {
    const editor = await EditorPage.open(
      'print("<b>bold</b> <color=red>red</color> <noparse><b></noparse>")\nprint("two")'
    );
    const { page } = editor;

    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('two');
    await page.locator('button.copy-terminal').click();

    await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe('bold red <b>\ntwo\n');
  });

  it('should wait between clearing the screen and printing again', async () => {
    const editor = await EditorPage.open(
      'for i in range(3)\n\twait(0.2)\n\tclear_screen\n\tprint(("#" * (3 - i)) + ("-" * i))\nend for'
    );
    const { page } = editor;

    const start = Date.now();

    await page.locator('button.run').click();
    await expect.poll(() => page.locator('button.run').isEnabled(), { timeout: 10_000 }).toBe(true);

    expect(Date.now() - start).toBeGreaterThanOrEqual(800);
    expect((await editor.getTerminalText()).trim()).toBe('###');
  });

  it('should read the input typed on the terminal', async () => {
    const editor = await EditorPage.open('name = user_input("Name: ")\nprint("hi " + name)');
    const { page } = editor;

    await page.locator('button.run').click();
    await expect.poll(() => editor.getTerminalText()).toContain('Name: ');
    await page.keyboard.type('Ada');
    await page.keyboard.press('Enter');

    await expect.poll(() => editor.getTerminalText()).toContain('hi Ada');
  });

  it('should stop at a breakpoint and run code where it stopped', async () => {
    const editor = await EditorPage.open('x = 21\ny = x * 2\nprint(y)');
    const { page } = editor;

    await editor.clickBreakpointGutter(2);
    await page.locator('button.debug').click();
    await page.locator('.debug-panel').waitFor();

    expect(await page.locator('.cm-stop-line').textContent()).toBe('y = x * 2');
    await page.locator('.debug-evaluate').fill('x + 1');
    await page.locator('.debug-evaluate').press('Enter');
    await expect.poll(() => page.locator('.debug-log').textContent()).toContain('22');
    await page.locator('button.debug-continue').click();

    await expect.poll(() => editor.getTerminalText()).toContain('42');
  });

  it('should transform the code', async () => {
    const editor = await EditorPage.open('value = 1 // a comment\nprint(value)');
    const { page } = editor;

    await expect.poll(() => page.locator('.transform-mode').textContent()).toBe('Uglify');
    await page.locator('button.transform').click();

    await expect.poll(() => page.locator('.transform-result').inputValue()).toMatch(/\bprint\([A-Za-z]\)$/);
    expect(await page.locator('.transform-result').inputValue()).not.toMatch(/comment|value/);
  });

  it('should share the code by a link', async () => {
    const editor = await EditorPage.open('print(1)');
    const { page } = editor;

    await page.locator('button.share').click();

    const link = new URL(await page.locator('.link-popup textarea').inputValue());

    expect(decodeURIComponent(atob(link.searchParams.get('c') ?? ''))).toBe('print(1)');
  });

  it('should insert the examples of the API reference into the code', async () => {
    const editor = await EditorPage.open('');
    const { page } = editor;

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
