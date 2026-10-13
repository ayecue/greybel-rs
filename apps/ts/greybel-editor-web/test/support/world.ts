import type { Page } from 'playwright';
import { expect } from 'vitest';

/** Writes the shared file with `params[0]` "write"; otherwise waits for a line of input, then prints the file. */
export const PROGRAM = [
  'computer = get_shell.host_computer',
  'if params.len > 0 and params[0] == "write" then',
  '  computer.touch(home_dir, "shared.txt")',
  '  computer.File(home_dir + "/shared.txt").set_content("from the writer")',
  '  print("written")',
  '  exit',
  'end if',
  'if params.len > 0 and params[0] == "wait" then user_input("Read? ")',
  'file = computer.File(home_dir + "/shared.txt")',
  'if file then print("content: " + file.get_content) else print("content: none")'
].join('\n');

/** Runs the project in the selected terminal with `params`, until it ends. */
export async function runToEnd(page: Page, params: string): Promise<void> {
  await page.locator('.params').fill(params);
  await page.locator('button.run').click();
  await expect.poll(() => page.locator('button.run').isEnabled(), { timeout: 10_000 }).toBe(true);
}

/** The terminal tab named `name`. */
export function terminalTab(page: Page, name: string) {
  return page.locator('.terminal-tab', { has: page.locator('.terminal-name', { hasText: name }) });
}
