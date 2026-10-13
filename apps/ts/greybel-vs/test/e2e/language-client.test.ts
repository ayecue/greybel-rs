import * as assert from 'assert';
import * as vscode from 'vscode';

import { TIMEOUT, diagnosticsOf, hoverAt, openFixture } from './support/workspace';

describe('language client', function () {
  this.timeout(TIMEOUT);

  it('should report the unresolved names of a document', async () => {
    const document = await openFixture('main.src');
    const diagnostics = await diagnosticsOf(document.uri);

    assert.deepStrictEqual(
      diagnostics.map((diagnostic) => diagnostic.message),
      ['Cannot find "nope".']
    );
  });

  it('should show the signature of a function from an include', async () => {
    const document = await openFixture('main.src');

    await diagnosticsOf(document.uri);

    const hover = await hoverAt(document, new vscode.Position(1, 7));

    assert.ok(
      hover.some((text) => text.includes('greet')),
      `hover: ${JSON.stringify(hover)}`
    );
  });
});
