import picomatch from 'picomatch';

import { EditorRouter, type FindFiles, type ReadFile } from '../editor.generated.js';

export class MemoryEditor extends EditorRouter {
  readonly files: Map<string, string>;

  constructor(files: Iterable<readonly [string, string]> = []) {
    super();
    this.files = new Map(files);
  }

  override onReadFile(request: ReadFile): string | null {
    return this.files.get(request.uri) ?? null;
  }

  override onFindFiles(request: FindFiles): string[] {
    const matchesInclude = picomatch(request.include);
    const matchesExclude = request.exclude == null ? () => false : picomatch(request.exclude);

    return [...this.files.keys()].filter((uri) => {
      const path = new URL(uri).pathname.replace(/^\//, '');

      return matchesInclude(path) && !matchesExclude(path);
    });
  }
}
