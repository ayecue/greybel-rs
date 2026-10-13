import { DEFAULT_PATH, type ProjectContent } from './content.js';
import { pathProblem } from './paths.js';

/**
 * Serializes multi-file projects as JSON documents marked as greybel projects. Serializes a single `main.src`
 * file as raw code for greybel-js compatibility, preserving existing links and saved code.
 */
interface ProjectDocument {
  readonly greybel: 'project';
  readonly version: 1;
  readonly entry: string;
  readonly files: Readonly<Record<string, string>>;
}

const MARKER = 'project';
const VERSION = 1;

/** Whether the project uses greybel-js single-file raw-code storage. */
function isSingleFile(project: ProjectContent): boolean {
  return project.files.size === 1 && project.entry === DEFAULT_PATH && project.files.has(DEFAULT_PATH);
}

/** Serialized project text. */
export function encodeProject(project: ProjectContent): string {
  if (isSingleFile(project)) return project.files.get(DEFAULT_PATH) ?? '';

  const document: ProjectDocument = {
    greybel: MARKER,
    version: VERSION,
    entry: project.entry,
    files: Object.fromEntries(project.files)
  };

  return JSON.stringify(document);
}

/** Deserializes `text` as a project document or as raw code for a single `main.src` file. */
export function decodeProject(text: string): ProjectContent {
  return projectDocument(text) ?? { entry: DEFAULT_PATH, files: new Map([[DEFAULT_PATH, text]]) };
}

/** Parses `text` as a supported project document with valid paths. */
function projectDocument(text: string): ProjectContent | undefined {
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
  const isValid = files.every(([path, code]) => typeof code === 'string' && pathProblem(path) === undefined);

  if (!isValid || files.length === 0) return undefined;

  return { entry: document.entry, files: new Map(files) };
}
