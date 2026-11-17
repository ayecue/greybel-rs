import type { PathResolver } from './paths.js';

export const DEFAULT_PATH = 'main.src';

/** Optional; `MANIFEST_TEMPLATE` applies until it exists. */
export const MANIFEST_PATH = 'greybel.json';

const MARKER = 'project';
const VERSION = 1;

interface ProjectDocument {
  readonly greybel: typeof MARKER;
  readonly version: typeof VERSION;
  readonly entry: string;
  readonly files: Readonly<Record<string, string>>;
}

/** A single `main.src` is stored as raw code, as greybel-js does; other projects as JSON. */
export class ProjectContent {
  readonly entry: string;
  readonly files: ReadonlyMap<string, string>;

  constructor(entry: string, files: ReadonlyMap<string, string>) {
    this.entry = entry;
    this.files = files;
  }

  static decode(text: string, paths: PathResolver): ProjectContent {
    return (
      ProjectContent.#parseDocument(text, paths) ?? new ProjectContent(DEFAULT_PATH, new Map([[DEFAULT_PATH, text]]))
    );
  }

  get isSingleFile(): boolean {
    return this.files.size === 1 && this.entry === DEFAULT_PATH && this.files.has(DEFAULT_PATH);
  }

  encode(): string {
    if (this.isSingleFile) return this.files.get(DEFAULT_PATH) ?? '';

    const document: ProjectDocument = {
      greybel: MARKER,
      version: VERSION,
      entry: this.entry,
      files: Object.fromEntries(this.files)
    };

    return JSON.stringify(document);
  }

  static #parseDocument(text: string, paths: PathResolver): ProjectContent | undefined {
    if (!text.startsWith('{')) return undefined;

    let value: unknown;

    try {
      value = JSON.parse(text);
    } catch {
      return undefined;
    }

    if (typeof value !== 'object' || value === null) return undefined;

    const document = value as Partial<ProjectDocument>;

    if (document.greybel !== MARKER || document.version !== VERSION || typeof document.entry !== 'string') {
      return undefined;
    }

    if (typeof document.files !== 'object' || document.files === null) return undefined;

    const files = Object.entries(document.files);
    const isValid = files.every(([path, code]) => typeof code === 'string' && paths.findProblem(path) === undefined);

    if (!isValid || files.length === 0) return undefined;

    return new ProjectContent(document.entry, new Map(files));
  }
}
