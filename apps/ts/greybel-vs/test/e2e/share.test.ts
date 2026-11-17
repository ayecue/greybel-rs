import * as assert from 'assert';
import { createServer } from 'http';
import * as vscode from 'vscode';

import { TIMEOUT, fixture } from './support/workspace';

/** Runs only when the extension is built with `EDITOR_SERVICE_URL=http://127.0.0.1:47123`. */
const SERVICE_URL = 'http://127.0.0.1:47123';

async function waitFor(check: () => boolean | Promise<boolean>): Promise<void> {
  for (let attempt = 0; attempt < 200; attempt++) {
    if (await check()) return;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  throw new Error('the condition did not hold in time');
}

describe('sharing a file', function () {
  this.timeout(TIMEOUT);

  it('should post the file with its imports as a workspace and copy its link', async function () {
    if (process.env.EDITOR_SERVICE_URL !== SERVICE_URL) this.skip();

    const bodies: unknown[] = [];
    const server = createServer((request, response) => {
      let body = '';

      request.on('data', (chunk: Buffer) => (body += chunk.toString()));
      request.on('end', () => {
        bodies.push({ url: request.url, json: JSON.parse(body) as unknown });
        response.setHeader('Content-Type', 'application/json');
        response.end(JSON.stringify({ id: 'a3f2c4d1-0000-4000-8000-000000000003' }));
      });
    });

    await new Promise<void>((resolve) => server.listen(47123, '127.0.0.1', resolve));

    try {
      const main = await vscode.workspace.openTextDocument(fixture('main.src'));
      const greet = await vscode.workspace.openTextDocument(fixture('lib/greet.src'));

      void vscode.commands.executeCommand('greybel.share', fixture('main.src'));
      await waitFor(() => bodies.length === 1);
      assert.deepStrictEqual(bodies[0], {
        url: '/workspaces',
        json: { entry: 'main.src', files: { 'main.src': main.getText(), 'lib/greet.src': greet.getText() } }
      });
      await waitFor(async () =>
        (await vscode.env.clipboard.readText()).endsWith('?id=a3f2c4d1-0000-4000-8000-000000000003')
      );
    } finally {
      server.close();
    }
  });
});
