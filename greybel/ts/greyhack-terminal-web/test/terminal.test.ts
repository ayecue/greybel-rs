import { describe, expect, it } from 'vitest';

import { WIDTH, openPage, recordedCase, screenshot } from './support/browser.js';
import { CASE_INDICES, MEAN, OVER_LIMIT } from './support/terminal.js';

describe('the terminal', () => {
  it.each(CASE_INDICES)('should draw recorded case %i the way Unity draws it', async (index) => {
    const record = recordedCase(index);
    const page = await openPage(WIDTH, record.height);

    await page.evaluate((source) => terminalPage.print(source), record.source);

    const shot = await screenshot(page);
    const difference = await page.evaluate(({ shot, capture }) => terminalPage.compare(shot, capture), {
      shot,
      capture: record.capture
    });

    expect(difference.mean, JSON.stringify(difference)).toBeLessThan(MEAN);
    expect(difference.overLimit, JSON.stringify(difference)).toBeLessThan(OVER_LIMIT);
  });

  it('should scroll up with the wheel', async () => {
    const page = await openPage(400, 100);

    await page.evaluate(() => terminalPage.print(Array.from({ length: 40 }, (_, line) => `line ${line}`).join('\n')));

    const beforeScroll = await screenshot(page);

    await page.mouse.move(200, 50);
    await page.evaluate(() => terminalPage.expectDraw());
    await page.mouse.wheel(0, -120);
    await page.evaluate(() => terminalPage.awaitDraw());
    expect(await screenshot(page)).not.toBe(beforeScroll);
  });
});
