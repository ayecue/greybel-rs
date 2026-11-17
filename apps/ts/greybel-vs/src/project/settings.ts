import vscode, { ConfigurationTarget, ExtensionContext, Uri, WorkspaceFolder } from 'vscode';

import { SECTION } from '../settings';

/** Project settings filename in a workspace folder. */
export const FILE_NAME = 'greybel.json';

/** List serialization for a `greybel.json` setting: comma-joined, space-joined, or first item. */
type Format = 'commaList' | 'spaceList' | 'first';

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type JsonObject = { [key: string]: Json };

class JsonPath {
  readonly #parts: readonly string[];

  constructor(parts: readonly string[]) {
    this.#parts = parts;
  }

  read(object: JsonObject): Json | undefined {
    let current: Json | undefined = object;

    for (const name of this.#parts) current = isObject(current) ? current[name] : undefined;

    return current;
  }

  write(object: JsonObject, value: Json): void {
    let current = object;

    for (const name of this.#parts.slice(0, -1)) {
      const next = current[name];

      current = isObject(next) ? next : (current[name] = {});
    }

    current[this.#parts[this.#parts.length - 1]!] = value;
  }

  /** Deletes the value and any objects left empty, returning whether a value existed. */
  delete(object: JsonObject): boolean {
    return this.#deleteParts(object, this.#parts);
  }

  #deleteParts(object: JsonObject, parts: readonly string[]): boolean {
    const [name, ...tail] = parts;
    const next = object[name!];

    if (tail.length === 0) return delete object[name!];
    if (!isObject(next) || !this.#deleteParts(next, tail)) return false;
    if (Object.keys(next).length === 0) delete object[name!];

    return true;
  }
}

/** A `greybel.json` path and its synchronized extension setting. */
class SettingBinding {
  /** Setting key under the `greybel` configuration section. */
  readonly key: string;
  readonly #path: JsonPath;
  readonly #format?: Format;

  constructor(path: readonly string[], setting: string, format: Json | undefined) {
    this.key = setting.slice(`${SECTION}.`.length);
    this.#path = new JsonPath(path);
    this.#format = typeof format === 'string' ? (format as Format) : undefined;
  }

  read(manifest: JsonObject): Json | undefined {
    return this.#path.read(manifest);
  }

  write(manifest: JsonObject, value: Json): void {
    this.#path.write(manifest, value);
  }

  delete(manifest: JsonObject): boolean {
    return this.#path.delete(manifest);
  }

  /** Converts a `greybel.json` value to its extension-setting representation. */
  toSetting(value: Json): Json {
    const items = Array.isArray(value) ? value.map(String) : [];

    switch (this.#format) {
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

  /** Converts a setting value to its `greybel.json` representation; variable values become text. */
  toManifestValue(value: Json): Json {
    const text = typeof value === 'string' ? value : '';

    switch (this.#format) {
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
              Object.entries(value).map(([name, item]) => [
                name,
                typeof item === 'string' ? item : JSON.stringify(item)
              ])
            )
          : value;
    }
  }
}

/** Bindings collected from `x-vscode-setting` in `schemas/greybel.schema.json`. */
class BindingTable {
  readonly #root: JsonObject;
  readonly #bindings: readonly SettingBinding[];

  constructor(root: JsonObject) {
    this.#root = root;
    this.#bindings = this.#collect(root, []);
  }

  get values(): readonly SettingBinding[] {
    return this.#bindings;
  }

  /** Collects bindings for `schema`'s properties under `path`, resolving `$ref` names through the root definitions. */
  #collect(schema: JsonObject, path: readonly string[]): SettingBinding[] {
    const properties = schema['properties'];

    if (!isObject(properties)) return [];

    return Object.entries(properties).flatMap(([name, property]): SettingBinding[] => {
      if (!isObject(property)) return [];

      const definition = this.#resolveDefinition(property);
      const setting = property['x-vscode-setting'];
      const at = [...path, name];

      if (typeof setting === 'string') {
        return [new SettingBinding(at, setting, property['x-vscode-format'])];
      }

      if (isObject(setting)) {
        return Object.entries(setting).map(([part, key]) => {
          const format = (definition['properties'] as JsonObject | undefined)?.[part];

          return new SettingBinding(
            [...at, part],
            key as string,
            isObject(format) ? format['x-vscode-format'] : undefined
          );
        });
      }

      return this.#collect(definition, at);
    });
  }

  #resolveDefinition(property: JsonObject): JsonObject {
    const reference = property['$ref'];
    const definitions = this.#root['$defs'];

    if (typeof reference !== 'string' || !isObject(definitions)) return property;

    const definition = definitions[reference.replace('#/$defs/', '')];

    return isObject(definition) ? definition : property;
  }
}

/**
 * Synchronizes project settings between workspace folders and their `greybel.json`. Workspace/folder setting
 * changes update `greybel.json`; file edits update folder settings. `greybel.json` takes precedence at startup.
 */
export class ProjectSettings {
  readonly #bindings: BindingTable;
  /** Queued synchronization operations, serialized to prevent stale reads from overwriting newer writes. */
  #syncs: Promise<void> = Promise.resolve();
  /** Last synchronized `greybel.json` values keyed by workspace-folder URI. */
  readonly #lastSync = new Map<string, JsonObject>();

  constructor(bindings: BindingTable) {
    this.#bindings = bindings;
  }

  static async load(context: ExtensionContext): Promise<ProjectSettings> {
    const schemaUri = Uri.joinPath(context.extensionUri, 'schemas', 'greybel.schema.json');
    const schema = JSON.parse(new TextDecoder().decode(await vscode.workspace.fs.readFile(schemaUri))) as JsonObject;

    return new ProjectSettings(new BindingTable(schema));
  }

  /** Extension settings for `uri` converted to `greybel.json` values for a document without that file. */
  createManifest(uri: Uri | undefined): JsonObject {
    const configuration = vscode.workspace.getConfiguration(SECTION, uri);
    const manifest: JsonObject = {};

    for (const binding of this.#bindings.values) {
      const value = configuration.get<Json>(binding.key);

      // A null setting is unset; greybel.json omits it.
      if (value === undefined || value === null) continue;
      binding.write(manifest, binding.toManifestValue(value));
    }

    return manifest;
  }

  /** Starts continuous synchronization for every workspace folder's settings and `greybel.json`. */
  start(context: ExtensionContext): void {
    const watcher = vscode.workspace.createFileSystemWatcher(`**/${FILE_NAME}`);
    const onFile = (uri: Uri) => {
      const folder = vscode.workspace.getWorkspaceFolder(uri);

      if (folder === undefined || this.#getFile(folder).toString() !== uri.toString()) return;
      this.#sync(() => this.#toSettings(folder));
    };

    context.subscriptions.push(
      watcher,
      watcher.onDidCreate(onFile),
      watcher.onDidChange(onFile),
      watcher.onDidDelete(onFile),
      vscode.workspace.onDidChangeConfiguration((event) => {
        for (const folder of vscode.workspace.workspaceFolders ?? []) {
          const changes = this.#bindings.values.filter((binding) =>
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

  /** Queues `sync` to run after all earlier synchronization operations complete. */
  #sync(sync: () => Promise<void>): void {
    this.#syncs = this.#syncs.then(sync).catch((error: unknown) => {
      vscode.window.showWarningMessage(`Could not sync settings with ${FILE_NAME}: ${String(error)}`);
    });
  }

  /**
   * Applies `folder`'s `greybel.json` values to differing folder settings and removes values deleted since
   * the last sync. A deleted file clears all synchronized values.
   */
  async #toSettings(folder: WorkspaceFolder): Promise<void> {
    const read = await this.#readManifest(folder);

    if (read === INVALID) return;

    const manifest = read ?? {};
    const lastSync = this.#lastSync.get(folder.uri.toString()) ?? {};
    const configuration = vscode.workspace.getConfiguration(SECTION, folder.uri);

    for (const binding of this.#bindings.values) {
      const value = binding.read(manifest);
      const setting = value === undefined ? undefined : binding.toSetting(value);
      const current = this.#getFolderValue(configuration, binding.key);

      if (value === undefined && (binding.read(lastSync) === undefined || current === undefined)) continue;
      if (value !== undefined && this.#equals(current, setting)) continue;
      await configuration.update(binding.key, setting, ConfigurationTarget.WorkspaceFolder);
    }

    this.#lastSync.set(folder.uri.toString(), manifest);
  }

  /**
   * Writes `changes` for `folder` to `greybel.json` using workspace/folder values, removing unset values.
   * Creates the file only for a defined value and skips invalid JSON.
   */
  async #toFile(folder: WorkspaceFolder, changes: readonly SettingBinding[]): Promise<void> {
    const configuration = vscode.workspace.getConfiguration(SECTION, folder.uri);
    const read = await this.#readManifest(folder);

    if (read === INVALID) return;

    const manifest = read ?? {};
    let dirty = false;

    for (const binding of changes) {
      const setting = this.#getFolderValue(configuration, binding.key);
      const current = binding.read(manifest);

      if (setting === undefined || setting === null) {
        dirty = (current !== undefined && binding.delete(manifest)) || dirty;
        continue;
      }

      const value = binding.toManifestValue(setting);

      if (this.#equals(current, value)) continue;
      binding.write(manifest, value);
      dirty = true;
    }

    if (!dirty) return;

    const text = `${JSON.stringify(manifest, null, 2)}\n`;

    this.#lastSync.set(folder.uri.toString(), manifest);
    await vscode.workspace.fs.writeFile(this.#getFile(folder), new TextEncoder().encode(text));
  }

  #getFile(folder: WorkspaceFolder): Uri {
    return Uri.joinPath(folder.uri, FILE_NAME);
  }

  /** Reads `folder`'s `greybel.json`; returns `undefined` if absent or `INVALID` if it is not a JSON object. */
  async #readManifest(folder: WorkspaceFolder): Promise<JsonObject | typeof INVALID | undefined> {
    let text: string;

    try {
      text = new TextDecoder().decode(await vscode.workspace.fs.readFile(this.#getFile(folder)));
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

  /** Returns the workspace or folder value of `key`, excluding user and default values. */
  #getFolderValue(configuration: vscode.WorkspaceConfiguration, key: string): Json | undefined {
    const inspection = configuration.inspect<Json>(key);

    return inspection?.workspaceFolderValue ?? inspection?.workspaceValue;
  }

  /** Returns whether `a` and `b` contain equivalent JSON regardless of key order. */
  #equals(a: Json | undefined, b: Json | undefined): boolean {
    return this.#toStableJson(a) === this.#toStableJson(b);
  }

  #toStableJson(value: Json | undefined): string {
    if (Array.isArray(value)) return `[${value.map((item) => this.#toStableJson(item)).join(',')}]`;
    if (isObject(value)) {
      return `{${Object.keys(value)
        .sort()
        .map((key) => `${JSON.stringify(key)}:${this.#toStableJson(value[key])}`)
        .join(',')}}`;
    }

    return JSON.stringify(value ?? null);
  }
}

/** Marker returned for a `greybel.json` that is not a JSON object. */
const INVALID = Symbol('invalid');

function isObject(value: Json | undefined): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
