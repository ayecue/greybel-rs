import { execFileSync } from 'node:child_process';

import { afterEach, beforeAll, describe, expect, inject, it } from 'vitest';

import { Build, Transform, TranspileMode } from '../src/index.js';
import { BUILD_TIMEOUT, LIBRARY, MAIN, children, settings, start } from './support/builder.js';

beforeAll(() => {
  execFileSync('cargo', ['build', '--quiet', '--package', 'greybel-builder-stdio'], {
    cwd: inject('rust'),
    stdio: 'inherit'
  });
}, BUILD_TIMEOUT);

afterEach(() => {
  for (const child of children.splice(0)) child.kill();
});

describe('the Rust builder over standard input and output', () => {
  it('should transform code on its own', async () => {
    const editor = start([]);

    const code = await editor.send(
      new Transform({
        code: 'a=1',
        transpile: settings(new TranspileMode.Beautify({ retainParentheses: false, indentationSpaces: 2 }))
      })
    );

    expect(code).toBe('a = 1');
  });

  it('should build a project with the files it imports, read from the editor', async () => {
    const editor = start([
      [MAIN, 'import_code("/project/lib.src")\nprint(double(4))'],
      [LIBRARY, 'double = function(n)\n  return n * 2\nend function']
    ]);

    const result = await editor.send(
      new Build({
        entries: [MAIN],
        extensions: ['src'],
        transpile: settings(new TranspileMode.Default()),
        ingameDirectory: '/root/project'
      })
    );

    expect(result.root).toBe('file:///project');
    expect(result.files.map((file) => file.path)).toEqual(['main.src', 'lib.src']);
  });
});
