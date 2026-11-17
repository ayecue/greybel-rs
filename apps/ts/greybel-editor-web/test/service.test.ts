import type { Page, Request } from 'playwright';
import { describe, expect, it } from 'vitest';

import { EditorPage } from './support/editor-page.js';

/** The `EDITOR_SERVICE_URL` of `.env.test`. */
const SERVICE_URL = 'http://editor-service.test';
const FILES = { 'main.src': 'import_code("lib/util.src")\nprint(x)', 'lib/util.src': 'x = 1' };

describe('saving to the editor service', () => {
  it('should save the files of the project as a workspace and show its link', async () => {
    const requests: Request[] = [];
    const editor = await EditorPage.open(EditorPage.encodeProject('main.src', FILES), async (page: Page) => {
      await page.route(`${SERVICE_URL}/workspaces`, async (route) => {
        requests.push(route.request());
        await route.fulfill({
          json: { id: 'a3f2c4d1-0000-4000-8000-000000000001', entry: 'main.src', files: FILES, createdAt: '' }
        });
      });
    });
    const { page } = editor;

    await page.locator('button.save').click();

    const link = page.locator('.link-popup textarea');

    await link.waitFor();
    expect(requests).toHaveLength(1);
    expect(requests[0]!.method()).toBe('POST');
    expect(requests[0]!.postDataJSON()).toMatchObject({ entry: 'main.src', files: FILES });
    expect(await link.inputValue()).toContain('?id=a3f2c4d1-0000-4000-8000-000000000001');
  });

  it("should show the service's message when it refuses to save", async () => {
    const editor = await EditorPage.open('print(1)', async (page: Page) => {
      await page.route(`${SERVICE_URL}/workspaces`, (route) =>
        route.fulfill({ status: 429, json: { message: 'Too many saves; try again in 30 seconds.' } })
      );
    });
    const { page } = editor;

    await page.locator('button.save').click();
    await expect
      .poll(() => page.locator('.notices').textContent())
      .toContain("The editor service didn't save the project: Too many saves; try again in 30 seconds.");
  });

  it('should open the workspace of an id link with all its files', async () => {
    const id = 'a3f2c4d1-0000-4000-8000-000000000002';
    const editor = await EditorPage.openWithQuery(new URLSearchParams({ id }), async (page: Page) => {
      await page.route(`${SERVICE_URL}/workspaces/${id}`, (route) =>
        route.fulfill({ json: { id, entry: 'main.src', files: FILES, createdAt: '' } })
      );
    });

    await expect.poll(() => editor.page.locator('.file-list li.file').count()).toBe(2);
    expect(await editor.getFileEntry('lib/util.src').count()).toBe(1);
    expect(await editor.page.locator('.cm-content').textContent()).toContain('import_code("lib/util.src")');
  });
});
