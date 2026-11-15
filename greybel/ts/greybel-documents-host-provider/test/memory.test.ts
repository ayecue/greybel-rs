import { describe, expect, it } from 'vitest';

import { FindFiles, ReadFile } from './server.generated.js';
import { connect, FILES } from './support/editor.js';

describe('MemoryEditor', () => {
  it('should read a file by its URI', async () => {
    const server = connect(FILES);
    const contents = server.send(new ReadFile({ uri: 'file:///work/main.src' }));

    await expect(contents).resolves.toBe('print 1');
  });

  it('should respond with null for a missing file', async () => {
    const server = connect(FILES);
    const contents = server.send(new ReadFile({ uri: 'file:///work/missing.src' }));

    await expect(contents).resolves.toBeNull();
  });

  it('should find the files matching a glob', async () => {
    const server = connect(FILES);
    const found = server.send(new FindFiles({ include: '**/*.src' }));

    await expect(found).resolves.toEqual(['file:///work/main.src', 'file:///work/lib/a.src']);
  });

  it('should not find the excluded files', async () => {
    const server = connect(FILES);
    const found = server.send(new FindFiles({ include: '**/*.src', exclude: '**/lib/**' }));

    await expect(found).resolves.toEqual(['file:///work/main.src']);
  });
});
