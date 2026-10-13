import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { LIBRARY_PROJECT, MISSING_PROJECT, session, uri } from './support/browser.js';

describe('greybel-lsp-web', () => {
  beforeAll(() => session.start());
  afterAll(() => session.close());

  it('should initialize as greybel-lsp', async () => {
    const name = await session.page.evaluate((project) => lspPage.serverName(project), LIBRARY_PROJECT);

    expect(name).toBe('greybel-lsp');
  });

  it('should read an include through the client', async () => {
    const openResult = await session.page.evaluate((project) => lspPage.open(project, 'main.src'), LIBRARY_PROJECT);

    expect(openResult.reads).toContain(uri('lib.src'));
  });

  it('should report no diagnostics for a name its include defines', async () => {
    const openResult = await session.page.evaluate((project) => lspPage.open(project, 'main.src'), LIBRARY_PROJECT);

    expect(openResult.diagnostics).toStrictEqual([]);
  });

  it('should report an unresolved reference', async () => {
    const openResult = await session.page.evaluate((project) => lspPage.open(project, 'main.src'), MISSING_PROJECT);

    expect(openResult.diagnostics.map(({ code, message }) => ({ code, message }))).toStrictEqual([
      { code: 'unresolved-reference', message: 'Cannot find "missing".' }
    ]);
  });

  it('should hover with the type an include gives a name, in markdown', async () => {
    const hover = await session.page.evaluate((project) => lspPage.hover(project, 'main.src', 1, 7), LIBRARY_PROJECT);

    expect(hover?.contents).toStrictEqual({ kind: 'markdown', value: '```greyscript\nhelper: number\n```' });
  });

  it.each(['{"jsonrpc":"2.0"}', 'not json'])(
    'should respond to %s, which is not a JSON-RPC message, with an invalid request error',
    async (json) => {
      const reply = await session.page.evaluate((message) => lspPage.reply(message), json);

      expect(reply).toStrictEqual({ jsonrpc: '2.0', id: null, error: { code: -32600, message: expect.any(String) } });
    }
  );

  it('should close the channel on exit after shutdown', async () => {
    const reason = await session.page.evaluate((project) => lspPage.exit(project), LIBRARY_PROJECT);

    expect(reason).toBe('ClosedChannel');
  });
});
