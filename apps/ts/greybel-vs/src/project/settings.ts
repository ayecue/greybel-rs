import vscode, { ConfigurationTarget, ExtensionContext, Uri, WorkspaceFolder } from 'vscode';

import { SECTION } from '../settings';

/** Project settings filename in a workspace folder. */
export const FILE_NAME = 'greybel.json';

/** List serialization for a `greybel.json` setting: comma-joined, space-joined, or first item. */
type Format = 'commaList' | 'spaceList' | 'first';

/** `greybel.json` setting at `path` and its synchronized extension setting. */
interface Binding {
  readonly path: readonly string[];
  /** Setting key under the `greybel` configuration section. */
  readonly key: string;
  readonly format?: Format;
}

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type JsonObject = { [key: string]: Json };

/**
 * Synchronizes project settings between workspace folders and their `greybel.json`. Workspace/folder setting
 * changes update `greybel.json`; file edits update folder settings. `greybel.json` takes precedence at
 * startup. Synchronized keys come from `x-vscode-setting` in `schemas/greybel.schema.json`.
 */
export class ProjectSettings {
  readonly #bindings: readonly Binding[];
  /** Queued synchronization operations, serialized to prevent stale reads from overwriting newer writes. */
  #syncs: Promise<void> = Promise.resolve();
  /** Last synchronized `greybel.json` values keyed by workspace-folder URI. */
  readonly #lastSync = new Map<string, JsonObject>();

  private constructor(bindings: readonly Binding[]) {
    this.#bindings = bindings;
  }

  static async load(context: ExtensionContext): Promise<ProjectSettings> {
    const schemaUri = Uri.joinPath(context.extensionUri, 'schemas', 'greybel.schema.json');
    const schema = JSON.parse(new TextDecoder().decode(await vscode.workspace.fs.readFile(schemaUri))) as JsonObject;

    return new ProjectSettings(bindingsOf(schema, schema, []));
  }

  /** Extension settings for `uri` converted to `greybel.json` values for a document without that file. */
  manifestOf(uri: Uri | undefined): JsonObject {
    const configuration = vscode.workspace.getConfiguration(SECTION, uri);
    const manifest: JsonObject = {};

    for (const binding of this.#bindings) {
      const value = configuration.get<Json>(binding.key);

      if (value === undefined) continue;
      setAt(manifest, binding.path, fromSetting(value, binding.format));
    }

    return manifest;
  }

  /** Starts continuous synchronization for every workspace folder's settings and `greybel.json`. */
  start(context: ExtensionContext): void {
    const watcher = vscode.workspace.createFileSystemWatcher(`**/${FILE_NAME}`);
    const onFile = (uri: Uri) => {
      const folder = vscode.workspace.getWorkspaceFolder(uri);

      if (folder === undefined || fileOf(folder).toString() !== uri.toString()) return;
      this.#sync(() => this.#toSettings(folder));
    };

    context.subscriptions.push(
      watcher,
      watcher.onDidCreate(onFile),
      watcher.onDidChange(onFile),
      watcher.onDidDelete(onFile),
      vscode.workspace.onDidChangeConfiguration((event) => {
        for (const folder of vscode.workspace.workspaceFolders ?? []) {
          const changes = this.#bindings.filter((binding) =>
            event.affectsConfiguration(`${SECTION}.${binding.key}`, folder.uri)
          );

          if (changes.length === 0) continue;
          this.#sync(() => this.#toFile(folder, changes));
        }
      }),
      vscode.workspace.onDidChangeWorkspaceFolders((event) =>
        event.added.forEach((folder) => this.#sync(() => this.#toSettings(folder)))
      )
    );

    for (const folder of vscode.workspace.workspaceFolders ?? []) this.#sync(() => this.#toSettings(folder));
  }

  /** Queue `sync` to run after all earlier synchronization operations complete. */
  #sync(sync: () => Promise<void>): void {
    this.#syncs = this.#syncs.then(sync).catch((error: unknown) => {
      vscode.window.showWarningMessage(`The settings and ${FILE_NAME} couldn't be synced: ${String(error)}`);
    });
  }

  /**
   * Applies `folder`'s `greybel.json` values to differing folder settings and removes values deleted since
   * the last sync. A deleted file clears all synchronized values.
   */
  async #toSettings(folder: WorkspaceFolder): Promise<void> {
    const read = await readManifest(folder);

    if (read === INVALID) return;

    const manifest = read ?? {};
    const lastSync = this.#lastSync.get(folder.uri.toString()) ?? {};
    const configuration = vscode.workspace.getConfiguration(SECTION, folder.uri);

    for (const binding of this.#bindings) {
      const value = getAt(manifest, binding.path);
      const setting = value === undefined ? undefined : toSetting(value, binding.format);
      const current = folderValue(configuration, binding.key);

      if (value === undefined && (getAt(lastSync, binding.path) === undefined || current === undefined)) continue;
      if (value !== undefined && equal(current, setting)) continue;
      await configuration.update(binding.key, setting, ConfigurationTarget.WorkspaceFolder);
    }

    this.#lastSync.set(folder.uri.toString(), manifest);
  }

  /**
   * Writes `changes` for `folder` to `greybel.json` using workspace/folder values, removing unset values.
   * Creates the file only for a defined value and leaves invalid JSON unchanged.
   */
  async #toFile(folder: WorkspaceFolder, changes: readonly Binding[]): Promise<void> {
    const configuration = vscode.workspace.getConfiguration(SECTION, folder.uri);
    const read = await readManifest(folder);

    if (read === INVALID) return;

    const manifest = read ?? {};
    let isChanged = false;

    for (const binding of changes) {
      const setting = folderValue(configuration, binding.key);
      const current = getAt(manifest, binding.path);

      if (setting === undefined) {
        isChanged = (current !== undefined && deleteAt(manifest, binding.path)) || isChanged;
        continue;
      }

      const value = fromSetting(setting, binding.format);

      if (equal(current, value)) continue;
      setAt(manifest, binding.path, value);
      isChanged = true;
    }

    if (!isChanged) return;

    const text = `${JSON.stringify(manifest, null, 2)}\n`;

    this.#lastSync.set(folder.uri.toString(), manifest);
    await vscode.workspace.fs.writeFile(fileOf(folder), new TextEncoder().encode(text));
  }
}

export function fileOf(folder: WorkspaceFolder): Uri {
  return Uri.joinPath(folder.uri, FILE_NAME);
}

/** Marker returned by `readManifest` for a `greybel.json` that is not a JSON object. */
const INVALID = Symbol('invalid');

/** Read `folder`'s `greybel.json`; return `undefined` if absent or `INVALID` if it is not a JSON object. */
async function readManifest(folder: WorkspaceFolder): Promise<JsonObject | typeof INVALID | undefined> {
  let text: string;

  try {
    text = new TextDecoder().decode(await vscode.workspace.fs.readFile(fileOf(folder)));
  } catch {
    return undefined;
  }

  let manifest: Json;

  try {
    manifest = JSON.parse(text) as Json;
  } catch {
    manifest = null;
  }

  if (isObject(manifest)) return manifest;
  vscode.window.showWarningMessage(`${FILE_NAME} of ${folder.name} isn't a JSON object: its settings aren't synced.`);

  return INVALID;
}

/** Return the workspace or folder value of `key`, excluding user and default values. */
function folderValue(configuration: vscode.WorkspaceConfiguration, key: string): Json | undefined {
  const inspection = configuration.inspect<Json>(key);

  return inspection?.workspaceFolderValue ?? inspection?.workspaceValue;
}

/** Collect bindings for `schema`'s properties under `path`, resolving `$ref` names through definitions in `root`. */
function bindingsOf(root: JsonObject, schema: JsonObject, path: readonly string[]): Binding[] {
  const properties = schema['properties'];

  if (!isObject(properties)) return [];

  return Object.entries(properties).flatMap(([name, property]): Binding[] => {
    if (!isObject(property)) return [];

    const definition = definitionOf(root, property);
    const setting = property['x-vscode-setting'];
    const at = [...path, name];

    if (typeof setting === 'string') return [binding(at, setting, property['x-vscode-format'])];
    if (isObject(setting)) {
      return Object.entries(setting).map(([part, key]) => {
        const format = (definition['properties'] as JsonObject | undefined)?.[part];

        return binding([...at, part], key as string, isObject(format) ? format['x-vscode-format'] : undefined);
      });
    }

    return bindingsOf(root, definition, at);
  });
}

function binding(path: readonly string[], setting: string, format: Json | undefined): Binding {
  return {
    path,
    key: setting.slice(`${SECTION}.`.length),
    format: typeof format === 'string' ? (format as Format) : undefined
  };
}

function definitionOf(root: JsonObject, property: JsonObject): JsonObject {
  const reference = property['$ref'];
  const definitions = root['$defs'];

  if (typeof reference !== 'string' || !isObject(definitions)) return property;

  const definition = definitions[reference.replace('#/$defs/', '')];

  return isObject(definition) ? definition : property;
}

/** Convert a `greybel.json` `value` to its extension-setting representation. */
function toSetting(value: Json, format: Format | undefined): Json {
  const items = Array.isArray(value) ? value.map(String) : [];

  switch (format) {
    case 'commaList':
      return items.join(',');
    case 'spaceList':
      return items.join(' ');
    case 'first':
      return items[0] ?? null;
    default:
      return value;
  }
}

/** Convert a setting `value` to its `greybel.json` representation; variable values become text. */
function fromSetting(value: Json, format: Format | undefined): Json {
  const text = typeof value === 'string' ? value : '';

  switch (format) {
    case 'commaList':
      return text
        .split(',')
        .map((item) => item.trim())
        .filter((item) => item !== '');
    case 'spaceList':
      return text.split(' ').filter((item) => item !== '');
    case 'first':
      return text === '' ? [] : [text];
    default:
      return isObject(value)
        ? Object.fromEntries(
            Object.entries(value).map(([name, item]) => [name, typeof item === 'string' ? item : JSON.stringify(item)])
          )
        : value;
  }
}

function getAt(object: JsonObject, path: readonly string[]): Json | undefined {
  let current: Json | undefined = object;

  for (const name of path) current = isObject(current) ? current[name] : undefined;

  return current;
}

function setAt(object: JsonObject, path: readonly string[], value: Json): void {
  let current = object;

  for (const name of path.slice(0, -1)) {
    const next = current[name];

    current = isObject(next) ? next : (current[name] = {});
  }

  current[path[path.length - 1]!] = value;
}

/** Delete the value at `path` and any objects left empty, returning whether a value existed. */
function deleteAt(object: JsonObject, path: readonly string[]): boolean {
  const [name, ...tail] = path;
  const next = object[name!];

  if (tail.length === 0) return delete object[name!];
  if (!isObject(next) || !deleteAt(next, tail)) return false;
  if (Object.keys(next).length === 0) delete object[name!];

  return true;
}

function isObject(value: Json | undefined): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Return whether `a` and `b` contain equivalent JSON regardless of key order. */
function equal(a: Json | undefined, b: Json | undefined): boolean {
  return stable(a) === stable(b);
}

function stable(value: Json | undefined): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
  if (isObject(value)) {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stable(value[key])}`)
      .join(',')}}`;
  }

  return JSON.stringify(value ?? null);
}
