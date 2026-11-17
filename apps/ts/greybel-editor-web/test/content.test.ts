import { describe, expect, it } from 'vitest';

import { ProjectContent } from '../src/project/content.js';
import { PathResolver } from '../src/project/paths.js';

const paths = new PathResolver();

describe('the text of a project', () => {
  it("should be the code of greybel-js's single file", () => {
    const single = new ProjectContent('main.src', new Map([['main.src', 'print(1)']]));

    expect(single.encode()).toBe('print(1)');
    expect(ProjectContent.decode('print(1)', paths)).toEqual(single);
  });

  it('should be a project document for several files', () => {
    const project = new ProjectContent(
      'lib/a.src',
      new Map([
        ['lib/a.src', 'import_code("/b.src")'],
        ['b.src', 'print(2)']
      ])
    );

    expect(ProjectContent.decode(project.encode(), paths)).toEqual(project);
  });

  it('should be code when it is JSON but no project document', () => {
    for (const text of ['{"a": 1}', '{"greybel":"project","version":2,"entry":"a","files":{"a":""}}', '{']) {
      expect(ProjectContent.decode(text, paths).files.get('main.src')).toBe(text);
    }
  });

  it('should be code when a path of the document is invalid', () => {
    const text = JSON.stringify({ greybel: 'project', version: 1, entry: 'a', files: { '../a': '' } });

    expect(ProjectContent.decode(text, paths).files.get('main.src')).toBe(text);
  });
});
