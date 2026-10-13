/**
 * Project storage used by the page: browser storage, link queries, and the editor service, compatible with
 * greybel-js.
 */

import { decodeProject, encodeProject } from './project/codec.js';
import type { ProjectContent } from './project/content.js';

/** greybel-js single-file storage key, read when no project is stored. */
const LEGACY_STORAGE_KEY = 'ide-content';
const STORAGE_KEY = 'greybel-editor-project';
/** Link query parameter for a project; retains compatibility with greybel-js single-file links. */
const CODE_PARAMETER = 'c';
const ID_PARAMETER = 'id';
const DEFAULT_CODE = 'print("Hello world")';

/** Editor service that saves code by ID, from greybel-js's EDITOR_SERVICE_URL; undefined when unavailable. */
const SERVICE_URL: string | undefined = import.meta.env.EDITOR_SERVICE_URL || undefined;

interface CodeRecord {
  readonly id: string;
  readonly content: string;
}

/** Initial page project: the linked saved project, linked shared project, or last edited project. */
export async function initialProject(location: Location): Promise<ProjectContent> {
  const parameters = new URLSearchParams(location.search);
  const id = parameters.get(ID_PARAMETER);

  if (id !== null && SERVICE_URL !== undefined) {
    try {
      const response = await fetch(`${SERVICE_URL}/code/${encodeURIComponent(id)}`);
      const record = (await response.json()) as CodeRecord;

      if (typeof record.content === 'string') return decodeProject(record.content);
    } catch (error) {
      console.error('greybel-editor-web: the saved project', error);
    }
  }

  const text =
    sharedText(parameters) ??
    localStorage.getItem(STORAGE_KEY) ??
    localStorage.getItem(LEGACY_STORAGE_KEY) ??
    DEFAULT_CODE;

  return decodeProject(text);
}

/** Store `project` as the most recently edited project. */
export function storeProject(project: ProjectContent): void {
  localStorage.setItem(STORAGE_KEY, encodeProject(project));
}

/** Return a link that shares `project` in its query using greybel-js's format: Base64 of the URI encoding of its text. */
export function shareLink(location: Location, project: ProjectContent): string {
  const url = new URL(location.href);

  url.search = '';
  url.searchParams.set(CODE_PARAMETER, btoa(encodeURIComponent(encodeProject(project))));

  return url.toString();
}

/** Indicates whether the page can save projects to the editor service. */
export const canSave = SERVICE_URL !== undefined;

/** Save `project` to the editor service and put its id in the page's address, producing the link that opens it. */
export async function saveProject(project: ProjectContent): Promise<string> {
  if (SERVICE_URL === undefined) throw new Error('The page has no editor service to save code to.');

  const response = await fetch(`${SERVICE_URL}/code`, {
    method: 'POST',
    body: JSON.stringify({ content: encodeProject(project) }),
    headers: { 'Content-Type': 'application/json' }
  });

  if (!response.ok) throw new Error(`The editor service didn't save the code (${response.status}).`);

  const record = (await response.json()) as CodeRecord;
  const url = new URL(location.href);

  url.search = '';
  url.searchParams.set(ID_PARAMETER, record.id);
  history.replaceState(null, '', url);

  return url.toString();
}

function sharedText(parameters: URLSearchParams): string | undefined {
  const parameter = parameters.get(CODE_PARAMETER);

  if (parameter === null) return undefined;

  try {
    return decodeURIComponent(atob(parameter));
  } catch (error) {
    console.error('greybel-editor-web: the shared project', error);

    return undefined;
  }
}
