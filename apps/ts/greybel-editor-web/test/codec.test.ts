import { describe, expect, it } from 'vitest';

import { decodeProject, encodeProject } from '../src/project/codec.js';

describe('the text of a project', () => {
  it("should be the code of greybel-js's single file", () => {
    const single = { entry: 'main.src', files: new Map([['main.src', 'print(1)']]) };

    expect(encodeProject(single)).toBe('print(1)');
    expect(decodeProject('print(1)')).toEqual(single);
  });

  it('should be a project document for several files', () => {
    const project = {
      entry: 'lib/a.src',
      files: new Map([
        ['lib/a.src', 'import_code("/b.src")'],
        ['b.src', 'print(2)']
      ])
    };

    expect(decodeProject(encodeProject(project))).toEqual(project);
  });

  it('should be code when it is JSON but no project document', () => {
    for (const text of ['{"a": 1}', '{"greybel":"project","version":2,"entry":"a","files":{"a":""}}', '{']) {
      expect(decodeProject(text).files.get('main.src')).toBe(text);
    }
  });

  it('should be code when a path of the document is invalid', () => {
    const text = JSON.stringify({ greybel: 'project', version: 1, entry: 'a', files: { '../a': '' } });

    expect(decodeProject(text).files.get('main.src')).toBe(text);
  });
});
