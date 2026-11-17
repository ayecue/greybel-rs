import { DEFAULT_PATH, MANIFEST_PATH, ProjectContent } from './project/content.js';
import type { ProjectEvent } from './project/event.js';
import { MANIFEST_TEMPLATE } from './project/manifest.js';
import type { PathResolver } from './project/paths.js';

export class Project {
  readonly paths: PathResolver;
  readonly #files: Map<string, string>;
  readonly #listeners = new Set<(event: ProjectEvent) => void>();

  #entry: string;

  constructor(content: ProjectContent, paths: PathResolver) {
    this.paths = paths;
    this.#files = new Map(content.files);
    if (this.scripts.length === 0) this.#files.set(DEFAULT_PATH, '');
    this.#entry = this.scripts.includes(content.entry) ? content.entry : this.scripts[0]!;
  }

  get entry(): string {
    return this.#entry;
  }

  get filePaths(): string[] {
    return [...this.#files.keys()].sort((left, right) => left.localeCompare(right));
  }

  /** All files except `greybel.json`. */
  get scripts(): string[] {
    return this.filePaths.filter((path) => path !== MANIFEST_PATH);
  }

  get content(): ProjectContent {
    return new ProjectContent(this.#entry, new Map(this.#files));
  }

  /** Keyed by URI. */
  get documents(): Map<string, string> {
    return new Map([...this.#files].map(([path, code]): [string, string] => [this.paths.toUri(path), code]));
  }

  has(path: string): boolean {
    return this.#files.has(path);
  }

  read(path: string): string | undefined {
    return this.#files.get(path);
  }

  subscribe(listener: (event: ProjectEvent) => void): () => void {
    this.#listeners.add(listener);

    return () => this.#listeners.delete(listener);
  }

  write(path: string, code: string): void {
    if (!this.#files.has(path)) return;
    this.#files.set(path, code);
    this.#notify({ kind: 'edit', path });
  }

  /** Uses the template for `greybel.json` when `code` is omitted. Returns an error message on failure. */
  create(path: string, code = path === MANIFEST_PATH ? MANIFEST_TEMPLATE : ''): string | undefined {
    const problem = this.#findNewPathProblem(path);

    if (problem !== undefined) return problem;
    this.#files.set(path, code);
    this.#notify({ kind: 'files', changes: [{ path, kind: 'creation' }] });

    return undefined;
  }

  /** Returns an error message on failure. */
  rename(from: string, to: string): string | undefined {
    const code = this.#files.get(from);

    if (code === undefined) return `There is no file ${from}.`;
    if (from === to) return undefined;
    if (from === MANIFEST_PATH) return `${MANIFEST_PATH} holds the project's settings and keeps its name.`;
    if (to === MANIFEST_PATH) return `${MANIFEST_PATH} is the name of the project's settings file.`;

    const problem = this.#findNewPathProblem(to);

    if (problem !== undefined) return problem;
    this.#files.delete(from);
    this.#files.set(to, code);
    if (this.#entry === from) this.#entry = to;
    this.#notify({
      kind: 'files',
      changes: [
        { path: from, kind: 'deletion' },
        { path: to, kind: 'creation' }
      ]
    });
    this.#notify({ kind: 'rename', from, to });

    return undefined;
  }

  /** Returns an error message on failure. */
  delete(path: string): string | undefined {
    if (!this.#files.has(path)) return `There is no file ${path}.`;
    if (path === MANIFEST_PATH) return `${MANIFEST_PATH} holds the project's settings and can't be deleted.`;
    if (this.scripts.length === 1) return 'A project has at least one file.';
    this.#files.delete(path);
    if (this.#entry === path) this.#entry = this.scripts[0]!;
    this.#notify({ kind: 'files', changes: [{ path, kind: 'deletion' }] });

    return undefined;
  }

  setEntry(path: string): void {
    if (!this.#files.has(path) || path === MANIFEST_PATH || this.#entry === path) return;
    this.#entry = path;
    this.#notify({ kind: 'entry' });
  }

  #findNewPathProblem(path: string): string | undefined {
    const problem = this.paths.findProblem(path);

    if (problem !== undefined) return problem;
    if (this.#files.has(path)) return `There is a file ${path} already.`;

    const isFolderOfFile = [...this.#files.keys()].some((other) => other.startsWith(`${path}/`));
    const isInFile = [...this.#files.keys()].some((other) => path.startsWith(`${other}/`));

    if (isFolderOfFile || isInFile) return `${path} is both a file and a folder.`;

    return undefined;
  }

  #notify(event: ProjectEvent): void {
    for (const listener of this.#listeners) listener(event);
  }
}
