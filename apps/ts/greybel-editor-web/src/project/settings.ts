import type { LSPClient } from '@codemirror/lsp-client';

import type { Project } from '../project.js';
import { MANIFEST_PATH } from './content.js';
import type { ProjectEvent } from './event.js';
import type { Json } from './json.js';
import type { LanguageServerSettings } from './language-server-settings.js';
import { MANIFEST_TEMPLATE, ResolvedSettings, type ResolvedSettingsJson } from './manifest.js';
import { ManifestProblem } from './manifest-problem.js';

const DELAY = 300;

/**
 * Reads `greybel.json` through greybel-lsp's `custom/project`. `current` retains the last valid settings while
 * `greybel.json` has an error. Reads are cached per folder and profile until a `greybel.json` or a non-script file
 * changes, since it may be an `#envar` file.
 */
export class ProjectSettings {
  /** Resolves after the first read, valid or not. */
  readonly ready: Promise<void>;

  readonly #client: LSPClient;
  readonly #project: Project;
  readonly #languageServerSettings: LanguageServerSettings;
  readonly #listeners = new Set<() => void>();
  readonly #unsubscribe: () => void;
  readonly #reads = new Map<string, Promise<ResolvedSettings>>();

  #current: ResolvedSettings | undefined;
  #problem: ManifestProblem | undefined;
  #timer: ReturnType<typeof setTimeout> | undefined;
  /** Discards the results of older refreshes. */
  #refreshes = 0;

  constructor(client: LSPClient, project: Project, languageServerSettings: LanguageServerSettings) {
    this.#client = client;
    this.#project = project;
    this.#languageServerSettings = languageServerSettings;
    this.#unsubscribe = project.subscribe((event) => {
      const paths = ProjectSettings.#getChangedPaths(event);

      if (paths.every((path) => this.#isScript(path))) return;
      this.#reads.clear();
      if (!paths.some((path) => project.paths.isManifest(path))) return;
      clearTimeout(this.#timer);
      this.#timer = setTimeout(() => void this.#refresh(), DELAY);
    });
    this.ready = this.#refresh();
  }

  get current(): ResolvedSettings | undefined {
    return this.#current;
  }

  get problem(): ManifestProblem | undefined {
    return this.#problem;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);

    return () => this.#listeners.delete(listener);
  }

  /** Throws the `greybel.json` error as an `Error`. */
  async read(path: string, profile?: string): Promise<ResolvedSettings> {
    try {
      return await this.#requestSettings(path, profile);
    } catch (error) {
      throw new Error(ManifestProblem.parse(ProjectSettings.#getErrorMessage(error), this.#project.paths).toString());
    }
  }

  close(): void {
    clearTimeout(this.#timer);
    this.#unsubscribe();
  }

  static #getChangedPaths(event: ProjectEvent): string[] {
    if (event.kind === 'edit') return [event.path];

    return event.kind === 'files' ? event.changes.map(({ path }) => path) : [];
  }

  #isScript(path: string): boolean {
    const extensions = this.#current?.manifest.extensions ?? ['src', 'gs', 'ms'];

    return !this.#project.paths.isManifest(path) && extensions.some((extension) => path.endsWith(`.${extension}`));
  }

  #requestSettings(path: string, profile: string | undefined): Promise<ResolvedSettings> {
    const key = `${path.slice(0, path.lastIndexOf('/') + 1)}\n${profile ?? ''}`;
    const previous = this.#reads.get(key);

    if (previous !== undefined) return previous;

    const read = this.#client.initializing
      .then(() =>
        this.#client.request<unknown, ResolvedSettingsJson>('custom/project', {
          uri: this.#project.paths.toUri(path),
          fallback: JSON.parse(MANIFEST_TEMPLATE) as Json,
          profile
        })
      )
      .then((json) => new ResolvedSettings(json));

    this.#reads.set(key, read);
    read.catch(() => this.#reads.delete(key));

    return read;
  }

  async #refresh(): Promise<void> {
    const refresh = ++this.#refreshes;

    try {
      const settings = await this.#requestSettings(MANIFEST_PATH, undefined);

      if (refresh !== this.#refreshes) return;
      this.#current = settings;
      this.#problem = undefined;
      this.#client.notification('workspace/didChangeConfiguration', {
        settings: { greybel: this.#languageServerSettings.fromManifest(settings.manifest) }
      });
    } catch (error) {
      if (refresh !== this.#refreshes) return;
      this.#problem = ManifestProblem.parse(ProjectSettings.#getErrorMessage(error), this.#project.paths);
    }

    for (const listener of this.#listeners) listener();
  }

  /** A JSON-RPC error object or an `Error` of the client. */
  static #getErrorMessage(error: unknown): string {
    if (typeof error === 'object' && error !== null && 'message' in error) return String(error.message);

    return String(error);
  }
}
