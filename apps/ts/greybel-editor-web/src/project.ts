import { DEFAULT_PATH, type ProjectContent } from './project/content.js';
import type { ProjectEvent } from './project/event.js';
import { fileUri, pathProblem } from './project/paths.js';

/**
 * Current project files and entry file. The language server reads files outside the editor, and the runtime
 * reads all files for each run.
 */
export class Project {
  readonly #files: Map<string, string>;
  readonly #listeners = new Set<(event: ProjectEvent) => void>();

  #entry: string;

  constructor(content: ProjectContent) {
    this.#files = new Map(content.files);
    if (this.#files.size === 0) this.#files.set(DEFAULT_PATH, '');
    this.#entry = this.#files.has(content.entry) ? content.entry : [...this.#files.keys()][0]!;
  }

  get entry(): string {
    return this.#entry;
  }

  /** Sorted project file paths. */
  get paths(): string[] {
    return [...this.#files.keys()].sort((left, right) => left.localeCompare(right));
  }

  get content(): ProjectContent {
    return { entry: this.#entry, files: new Map(this.#files) };
  }

  /** Project file contents keyed by URI. */
  get documents(): Map<string, string> {
    return new Map([...this.#files].map(([path, code]): [string, string] => [fileUri(path), code]));
  }

  has(path: string): boolean {
    return this.#files.has(path);
  }

  /** Contents of `path`, or undefined if the file does not exist. */
  read(path: string): string | undefined {
    return this.#files.get(path);
  }

  /** Registers `listener` for project changes. */
  subscribe(listener: (event: ProjectEvent) => void): () => void {
    this.#listeners.add(listener);

    return () => this.#listeners.delete(listener);
  }

  /** Replaces the edited contents of `path`. */
  write(path: string, code: string): void {
    if (!this.#files.has(path)) return;
    this.#files.set(path, code);
    this.#notify({ kind: 'edit', path });
  }

  /** Adds a file at `path`; returns an error on failure. */
  create(path: string, code = ''): string | undefined {
    const problem = this.#newPathProblem(path);

    if (problem !== undefined) return problem;
    this.#files.set(path, code);
    this.#notify({ kind: 'files', changes: [{ path, kind: 'creation' }] });

    return undefined;
  }

  /** Moves the file at `from` to `to`; why it cannot, when it cannot. */
  rename(from: string, to: string): string | undefined {
    const code = this.#files.get(from);

    if (code === undefined) return `There is no file ${from}.`;
    if (from === to) return undefined;

    const problem = this.#newPathProblem(to);

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

    return undefined;
  }

  /** Delete the file at `path` and return the reason on failure; a project must retain at least one file. */
  delete(path: string): string | undefined {
    if (!this.#files.has(path)) return `There is no file ${path}.`;
    if (this.#files.size === 1) return 'A project has at least one file.';
    this.#files.delete(path);
    if (this.#entry === path) this.#entry = this.paths[0]!;
    this.#notify({ kind: 'files', changes: [{ path, kind: 'deletion' }] });

    return undefined;
  }

  /** Set the file at `path` as the entry point for runs. */
  setEntry(path: string): void {
    if (!this.#files.has(path) || this.#entry === path) return;
    this.#entry = path;
    this.#notify({ kind: 'entry' });
  }

  #newPathProblem(path: string): string | undefined {
    const problem = pathProblem(path);

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
