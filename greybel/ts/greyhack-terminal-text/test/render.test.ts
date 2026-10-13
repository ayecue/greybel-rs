import { execFileSync } from 'node:child_process';

import { afterEach, beforeAll, describe, expect, inject, it } from 'vitest';

import { Render, RichText } from '../src/index.js';
import { BUILD_TIMEOUT, processes, start } from './support/runtime.js';

beforeAll(() => {
  execFileSync('cargo', ['build', '--quiet', '--package', 'greybel-runtime-stdio'], {
    cwd: inject('rust'),
    stdio: 'inherit'
  });
}, BUILD_TIMEOUT);

afterEach(() => {
  for (const child of processes.splice(0)) child.kill();
});

describe('the renderer of the Rust runtime over standard input and output', () => {
  it('should render rich text as ANSI escape sequences, laid out at the columns', async () => {
    const terminal = start();

    const output = await terminal.send(
      new Render({
        texts: [
          new RichText({ text: '<color=#FF0000>red</color>', columns: 0 }),
          new RichText({ text: 'one two three four five six seven', columns: 12 })
        ]
      })
    );

    expect(output.texts).toEqual(['\x1b[38;2;255;0;0mred\x1b[0m', 'one two\nthree four\nfive six\nseven']);
  });
});
