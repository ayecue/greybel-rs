import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';

const PROGRAM = [
  'user = {"name": "rex", "tags": [1, 2]}',
  'count = 3',
  'double = function(value)',
  '  result = value * 2',
  '  debugger',
  '  return result',
  'end function',
  'total = double(count)',
  'print(user.name + " " + total)'
].join('\n');

async function openStopped(code: string = PROGRAM): Promise<EditorPage> {
  const editor = await EditorPage.open(code);

  await editor.page.locator('button.debug').click();
  await editor.waitForStop();

  return editor;
}

describe('the debugger', () => {
  it('should say why and where the program stopped', async () => {
    const editor = await openStopped();

    expect(await editor.page.locator('.debug-status').textContent()).toBe(
      'Paused on debugger statement at main.src:5:3'
    );
  });

  it('should stay open with its console while the program runs between stops', async () => {
    const editor = await openStopped('x = 1\ndebugger\nx = 2\ndebugger\nprint(x)');
    const { page } = editor;

    await page.locator('.debug-evaluate').fill('x');
    await page.locator('.debug-evaluate').press('Enter');
    await expect.poll(() => page.locator('.debug-log').textContent()).toContain('> x1');
    await page.locator('button.debug-continue').click();
    await expect
      .poll(() => page.locator('.debug-status').textContent())
      .toBe('Paused on debugger statement at main.src:4:1');

    expect(await page.locator('.debug-log').textContent()).toContain('> x1');
    expect(await page.locator('.execute-panel button.stop').count()).toBe(0);
    await page.locator('.debug-evaluate').press('ArrowUp');
    expect(await page.locator('.debug-evaluate').inputValue()).toBe('x');
  });

  it("should show a caller frame's variables and mark its line", async () => {
    const editor = await openStopped();
    const { page } = editor;
    const variables = page.locator('.debug-variables-section');

    await expect.poll(() => variables.textContent()).toContain('value3number');
    await page.locator('.call-frame').nth(1).click();

    await expect.poll(() => variables.textContent()).toContain('count3number');
    expect(await page.locator('.cm-frame-line').textContent()).toBe('total = double(count)');
    expect(await page.locator('.cm-stop-line').textContent()).toBe('  debugger');
  });

  it('should open lists and maps and keep them open after a step', async () => {
    const editor = await openStopped();
    const { page } = editor;
    const user = page.locator('.value-node[data-path="Globals/user"]');

    await page.locator('.debug-scope summary', { hasText: 'Globals' }).click();
    await user.locator('.value-toggle').first().click();
    await expect.poll(() => user.locator('.value-children').textContent()).toContain('name"rex"string');
    await page.locator('button.debug-step-over').click();
    await expect.poll(() => page.locator('.debug-status').textContent()).toContain('Paused after step');

    await expect.poll(() => user.locator('.value-children').textContent()).toContain('name"rex"string');
  });

  it('should change a value by double-click', async () => {
    const editor = await openStopped();
    const { page } = editor;
    const result = page.locator('.value-node[data-path="Locals/result"]');

    await result.locator('.value-text').dblclick();
    await result.locator('.value-edit').fill('40 + 2');
    await result.locator('.value-edit').press('Enter');
    await expect.poll(() => result.locator('.value-text').textContent()).toBe('42');
    await page.locator('button.debug-continue').click();

    await expect.poll(() => editor.getTerminalText()).toContain('rex 42');
  });

  it('should evaluate watch expressions at every stop', async () => {
    const editor = await openStopped('x = 1\ndebugger\nx = 2\ndebugger\nprint(x)');
    const { page } = editor;

    await page.locator('.watch-add').fill('x * 10');
    await page.locator('.watch-add').press('Enter');
    await expect.poll(() => page.locator('.watch-view').textContent()).toContain('x * 1010number');
    await page.locator('button.debug-continue').click();

    await expect.poll(() => page.locator('.watch-view').textContent()).toContain('x * 1020number');
    await page.locator('.watch-remove').click();
    expect(await page.locator('.watch-entry').count()).toBe(0);
  });

  it('should stop the program from the panel', async () => {
    const editor = await openStopped();
    const { page } = editor;

    await page.locator('button.debug-stop').click();

    await expect.poll(() => page.locator('.debug-panel').count()).toBe(0);
    expect(await editor.getTerminalText()).toContain('The program was stopped.');
  });
});
