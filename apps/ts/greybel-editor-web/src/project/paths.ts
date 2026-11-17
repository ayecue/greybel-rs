import { URI } from 'vscode-uri';

import { MANIFEST_PATH } from './content.js';

/** Converts project paths such as `lib/util.src` to file URIs under `file:///` and back. */
export class PathResolver {
  readonly #root = URI.file('/');

  get rootUri(): string {
    return this.#root.toString();
  }

  toUri(path: string): string {
    return URI.file(`/${path}`).toString();
  }

  /** Undefined outside the project. */
  toPath(uri: string): string | undefined {
    let target: URI;

    try {
      target = URI.parse(uri, true);
    } catch {
      return undefined;
    }

    if (target.scheme !== this.#root.scheme || target.authority !== '' || !target.path.startsWith('/')) {
      return undefined;
    }

    return target.path.slice(1);
  }

  /** Returns an error message for an invalid path. */
  findProblem(path: string): string | undefined {
    if (path === '') return 'A file needs a name.';
    if (path.startsWith('/') || path.endsWith('/')) return 'A path neither starts nor ends with a slash.';

    const segments = path.split('/');

    if (segments.some((segment) => segment === '' || segment === '.' || segment === '..')) {
      return 'A path has no empty, "." or ".." folder.';
    }

    return undefined;
  }

  normalize(text: string): string {
    return text.trim().replaceAll('\\', '/');
  }

  /** Also matches the `greybel.json` of a nested folder. */
  isManifest(path: string): boolean {
    return path === MANIFEST_PATH || path.endsWith(`/${MANIFEST_PATH}`);
  }
}
