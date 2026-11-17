import { isObject, type Json, type JsonObject } from './json.js';
import type { Manifest } from './manifest.js';

interface Binding {
  readonly path: readonly string[];
  readonly key: string;
  readonly format: string | undefined;
}

/**
 * Maps `greybel.json` to greybel-lsp settings by `x-vscode-setting` and `x-vscode-format` in the schema, as greybel-vs
 * does.
 */
export class LanguageServerSettings {
  readonly #schema: JsonObject;
  readonly #bindings: readonly Binding[];

  constructor(schema: JsonObject) {
    this.#schema = schema;
    this.#bindings = this.#collectBindings(schema, []);
  }

  fromManifest(manifest: Manifest): JsonObject {
    const settings: JsonObject = {};

    for (const { path, key, format } of this.#bindings) {
      const value = this.#getValueAt(manifest as unknown as JsonObject, path);

      if (value === undefined) continue;
      this.#setAt(settings, key.split('.'), this.#formatValue(value, format));
    }

    return settings;
  }

  #collectBindings(schema: JsonObject, path: readonly string[]): Binding[] {
    const properties = schema['properties'];

    if (!isObject(properties)) return [];

    return Object.entries(properties).flatMap(([name, property]): Binding[] => {
      if (!isObject(property)) return [];

      const definition = this.#resolveDefinition(property);
      const setting = property['x-vscode-setting'];
      const at = [...path, name];

      if (typeof setting === 'string') return [this.#createBinding(at, setting, property['x-vscode-format'])];
      if (isObject(setting)) {
        return Object.entries(setting).map(([part, key]) => {
          const format = (definition['properties'] as JsonObject | undefined)?.[part];

          return this.#createBinding(
            [...at, part],
            key as string,
            isObject(format) ? format['x-vscode-format'] : undefined
          );
        });
      }

      return this.#collectBindings(definition, at);
    });
  }

  #createBinding(path: readonly string[], setting: string, format: Json | undefined): Binding {
    return { path, key: setting.replace(/^greybel\./, ''), format: typeof format === 'string' ? format : undefined };
  }

  #resolveDefinition(property: JsonObject): JsonObject {
    const reference = property['$ref'];
    const definitions = this.#schema['$defs'];

    if (typeof reference !== 'string' || !isObject(definitions)) return property;

    const definition = definitions[reference.replace('#/$defs/', '')];

    return isObject(definition) ? definition : property;
  }

  #formatValue(value: Json, format: string | undefined): Json {
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

  #getValueAt(object: JsonObject, path: readonly string[]): Json | undefined {
    let current: Json | undefined = object;

    for (const name of path) current = isObject(current) ? current[name] : undefined;

    return current;
  }

  #setAt(object: JsonObject, path: readonly string[], value: Json): void {
    let current = object;

    for (const name of path.slice(0, -1)) {
      const next = current[name];

      current = isObject(next) ? next : (current[name] = {});
    }

    current[path[path.length - 1]!] = value;
  }
}
