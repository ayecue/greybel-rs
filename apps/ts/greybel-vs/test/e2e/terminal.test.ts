import * as assert from 'assert';
import { Output, OutputBatch } from 'greybel-io-web';

import { RecordedTerminal } from './support/terminal';
import { TIMEOUT } from './support/workspace';

describe("the program's terminal", function () {
  this.timeout(TIMEOUT);

  let terminal: RecordedTerminal;

  beforeEach(() => {
    terminal = new RecordedTerminal();
  });

  afterEach(() => {
    terminal.dispose();
  });

  it('should show rich text as the runtime renders it, in the order it was printed', async () => {
    terminal.page.onOutputBatch(
      new OutputBatch({
        entries: [
          new Output.Print({ text: '<color=#FF0000>red</color>', newLine: true, replace: false }),
          new Output.Print({ text: '<b>bold</b>', newLine: true, replace: false })
        ]
      })
    );
    terminal.page.printError('failure');

    await terminal.sent('failure');
    assert.strictEqual(
      terminal.written.join(''),
      '\x1b[38;2;255;0;0mred\x1b[0m\r\n\x1b[1mbold\x1b[0m\r\n\x1b[31mfailure\x1b[0m\r\n'
    );
  });
});
