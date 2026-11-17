import { ProjectContent } from './content.js';
import type { PathResolver } from './paths.js';

/** greybel-js storage key; read when no project is stored. */
const LEGACY_STORAGE_KEY = 'ide-content';
const STORAGE_KEY = 'greybel-editor-project';
/** Same parameter as greybel-js links. */
const CODE_PARAMETER = 'c';
const ID_PARAMETER = 'id';
const DEFAULT_CODE = 'print("Hello world")';

/** A workspace of greybel-editor-service: `GET /workspaces/{id}` and `POST /workspaces`. */
interface WorkspaceRecord {
  readonly id: string;
  readonly entry: string;
  readonly files: Readonly<Record<string, string>>;
}

/** Stores projects in local storage, in share links, and in greybel-editor-service when `serviceUrl` is set. */
export class ProjectStorage {
  readonly #location: Location;
  readonly #storage: Storage;
  readonly #serviceUrl: string | undefined;
  readonly #paths: PathResolver;

  constructor(location: Location, storage: Storage, serviceUrl: string | undefined, paths: PathResolver) {
    this.#location = location;
    this.#storage = storage;
    this.#serviceUrl = serviceUrl;
    this.#paths = paths;
  }

  get canSave(): boolean {
    return this.#serviceUrl !== undefined;
  }

  /** The project of the page's link, else the last stored one. */
  async loadInitial(): Promise<ProjectContent> {
    const parameters = new URLSearchParams(this.#location.search);
    const id = parameters.get(ID_PARAMETER);

    if (id !== null && this.#serviceUrl !== undefined) {
      try {
        const content = await this.#loadWorkspace(id);

        if (content !== undefined) return content;
      } catch (error) {
        console.error('greybel-editor-web: the saved project', error);
      }
    }

    const text =
      this.#readSharedText(parameters) ??
      this.#storage.getItem(STORAGE_KEY) ??
      this.#storage.getItem(LEGACY_STORAGE_KEY) ??
      DEFAULT_CODE;

    return ProjectContent.decode(text, this.#paths);
  }

  store(content: ProjectContent): void {
    this.#storage.setItem(STORAGE_KEY, content.encode());
  }

  /** greybel-js link format: Base64 of the URI-encoded text. */
  createShareLink(content: ProjectContent): string {
    const url = new URL(this.#location.href);

    url.search = '';
    url.searchParams.set(CODE_PARAMETER, btoa(encodeURIComponent(content.encode())));

    return url.toString();
  }

  /** Also sets the saved id in the page's URL. */
  async save(content: ProjectContent): Promise<string> {
    if (this.#serviceUrl === undefined) throw new Error('The page has no editor service to save code to.');

    const response = await fetch(`${this.#serviceUrl}/workspaces`, {
      method: 'POST',
      body: JSON.stringify({ entry: content.entry, files: Object.fromEntries(content.files) }),
      headers: { 'Content-Type': 'application/json' }
    });

    if (!response.ok) {
      throw new Error(`The editor service didn't save the project: ${await this.#getServiceMessage(response)}`);
    }

    const record = (await response.json()) as WorkspaceRecord;
    const url = new URL(this.#location.href);

    url.search = '';
    url.searchParams.set(ID_PARAMETER, record.id);
    history.replaceState(null, '', url);

    return url.toString();
  }

  async #loadWorkspace(id: string): Promise<ProjectContent | undefined> {
    const response = await fetch(`${this.#serviceUrl}/workspaces/${encodeURIComponent(id)}`);

    if (!response.ok) throw new Error(await this.#getServiceMessage(response));

    const record = (await response.json()) as WorkspaceRecord;
    const files = Object.entries(record.files);
    const isValid = files.every(
      ([path, code]) => typeof code === 'string' && this.#paths.findProblem(path) === undefined
    );

    if (!isValid || !files.some(([path]) => path === record.entry)) return undefined;

    return new ProjectContent(record.entry, new Map(files));
  }

  /** The service's `{"message"}`, or the status when the body has none. */
  async #getServiceMessage(response: Response): Promise<string> {
    const body = (await response.json().catch(() => undefined)) as { message?: unknown } | undefined;

    return typeof body?.message === 'string' ? body.message : `status ${response.status}`;
  }

  #readSharedText(parameters: URLSearchParams): string | undefined {
    const parameter = parameters.get(CODE_PARAMETER);

    if (parameter === null) return undefined;

    try {
      return decodeURIComponent(atob(parameter));
    } catch (error) {
      console.error('greybel-editor-web: the shared project', error);

      return undefined;
    }
  }
}
