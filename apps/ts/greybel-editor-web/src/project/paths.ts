/** Project root: `main.src` maps to `file:///main.src` and imports use `/main.src`. */
const ROOT = 'file:///';

/** Validation error for a project file path, or undefined when valid. */
export function pathProblem(path: string): string | undefined {
  if (path === '') return 'A file needs a name.';
  if (path.startsWith('/') || path.endsWith('/')) return 'A path neither starts nor ends with a slash.';

  const segments = path.split('/');

  if (segments.some((segment) => segment === '' || segment === '.' || segment === '..')) {
    return 'A path has no empty, "." or ".." folder.';
  }

  return undefined;
}

/** Normalizes `path` by trimming whitespace and converting backslashes to slashes. */
export function normalizePath(path: string): string {
  return path.trim().replaceAll('\\', '/');
}

/** Returns the project file URI for `path`. */
export function fileUri(path: string): string {
  return new URL(path.split('/').map(encodeURIComponent).join('/'), ROOT).href;
}

/** Project root URI used to resolve absolute imports. */
export const rootUri = ROOT;

/** Project-relative path for `uri`, or undefined when outside the project. */
export function filePath(uri: string): string | undefined {
  if (!uri.startsWith(ROOT)) return undefined;

  try {
    return decodeURIComponent(new URL(uri).pathname.slice(1));
  } catch {
    return undefined;
  }
}
