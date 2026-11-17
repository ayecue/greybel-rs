import type { Completion, CompletionContext, CompletionResult } from '@codemirror/autocomplete';
import { json } from '@codemirror/lang-json';
import { syntaxTree } from '@codemirror/language';
import { setDiagnostics, type Diagnostic } from '@codemirror/lint';
import type { EditorState, Extension, TransactionSpec } from '@codemirror/state';
import { hoverTooltip, type Tooltip } from '@codemirror/view';
import type { SyntaxNode } from '@lezer/common';

import { asObject, type Json, type JsonObject } from '../project/json.js';
import type { ManifestProblem } from '../project/manifest-problem.js';

/** `*` in `path` stands for a list item. */
interface Place {
  readonly path: readonly string[];
  readonly kind: 'name' | 'value';
  readonly node: SyntaxNode | undefined;
  readonly names: readonly string[];
}

/** JSON language, schema completion and hovers for `greybel.json`. */
export class ManifestHelp {
  readonly #schema: JsonObject;

  constructor(schema: JsonObject) {
    this.#schema = schema;
  }

  extension(): Extension {
    return [
      json(),
      json().language.data.of({ autocomplete: (context: CompletionContext) => this.#complete(context) }),
      hoverTooltip((view, pos) => this.#createHover(view.state, pos))
    ];
  }

  createProblemUpdate(state: EditorState, problem: ManifestProblem | undefined): TransactionSpec {
    const diagnostics: Diagnostic[] = [];

    if (problem !== undefined) {
      const line = state.doc.line(Math.min(Math.max(problem.line, 1), state.doc.lines));
      const indentation = /^\s*/.exec(line.text)?.[0].length ?? 0;

      diagnostics.push({ from: line.from + indentation, to: line.to, severity: 'error', message: problem.message });
    }

    return setDiagnostics(state, diagnostics);
  }

  #complete(context: CompletionContext): CompletionResult | null {
    const place = this.#findPlace(context.state, context.pos);

    if (place === undefined) return null;

    const schema = this.#getSchemaAt(place.path);

    if (schema === undefined) return null;

    const inString = place.node?.name === 'String' || place.node?.name === 'PropertyName';
    const from = inString ? place.node!.from : context.pos;
    const to = inString ? place.node!.to : context.pos;

    if (!inString && !context.explicit && place.kind === 'name') {
      const before = context.state.sliceDoc(Math.max(0, context.pos - 1), context.pos);

      if (!/[{,\s]/.test(before)) return null;
    }

    const options =
      place.kind === 'name' ? this.#getNameCompletions(schema, place, inString) : this.#getValueCompletions(schema);

    return options.length === 0 ? null : { from, to, options, filter: true };
  }

  #getNameCompletions(schema: JsonObject, place: Place, inString: boolean): Completion[] {
    const properties = asObject(schema['properties']) ?? {};
    const propertyNames = asObject(schema['propertyNames']);
    const names = [
      ...Object.keys(properties),
      ...(Array.isArray(propertyNames?.['enum']) ? propertyNames['enum'].map(String) : [])
    ].filter((name) => name !== '$schema' && !place.names.includes(name));

    return names.map((name) => {
      const property = this.#resolveReference(asObject(properties[name]));

      return {
        label: name,
        type: 'property',
        detail: property === undefined ? undefined : this.#describeType(property),
        info: this.#getDescription(property),
        apply: inString ? JSON.stringify(name) : `${JSON.stringify(name)}: `
      };
    });
  }

  #getValueCompletions(schema: JsonObject): Completion[] {
    const completions = this.#getChoices(schema).map(([value, description]) => ({
      label: JSON.stringify(value),
      type: 'enum',
      info: description
    }));
    const types = this.#getTypes(schema);

    if (types.includes('boolean')) {
      completions.push({ label: 'true', type: 'keyword', info: undefined });
      completions.push({ label: 'false', type: 'keyword', info: undefined });
    }

    return completions;
  }

  #createHover(state: EditorState, pos: number): Tooltip | null {
    const node = syntaxTree(state).resolveInner(pos, 1);
    const target = node.name === 'PropertyName' || node.name === 'String' ? node : undefined;

    if (target === undefined) return null;

    const place = this.#findPlace(state, target.from + 1);

    if (place === undefined) return null;

    const text = JSON.parse(state.sliceDoc(target.from, target.to)) as string;
    const lines =
      place.kind === 'name'
        ? this.#describeName(place.path, text)
        : this.#describeValue(this.#getSchemaAt(place.path), text);

    if (lines === undefined) return null;

    return {
      pos: target.from,
      end: target.to,
      above: true,
      create: () => {
        const dom = document.createElement('div');

        dom.className = 'cm-manifest-hover';
        for (const [index, line] of lines.entries()) {
          const element = document.createElement(index === 0 ? 'code' : 'p');

          element.textContent = line;
          dom.append(element);
        }

        return { dom };
      }
    };
  }

  #describeName(path: readonly string[], name: string): string[] | undefined {
    const schema = this.#getSchemaAt([...path, name]);

    if (schema === undefined) return undefined;

    const lines = [`${name}: ${this.#describeType(schema)}`];
    const description = this.#getDescription(schema);
    const fallback = schema['default'];

    if (description !== undefined) lines.push(description);
    if (fallback !== undefined) lines.push(`Default: ${JSON.stringify(fallback)}`);

    return lines;
  }

  #describeValue(schema: JsonObject | undefined, value: string): string[] | undefined {
    const choice = schema === undefined ? undefined : this.#getChoices(schema).find(([other]) => other === value);

    if (choice === undefined || choice[1] === undefined) return undefined;

    return [JSON.stringify(value), choice[1]];
  }

  /** Undefined outside the top-level object. */
  #findPlace(state: EditorState, pos: number): Place | undefined {
    const inner = syntaxTree(state).resolveInner(pos, -1);

    if ((inner.name === '}' || inner.name === ']') && pos >= inner.to) return undefined;

    const path: string[] = [];
    let kind: Place['kind'] | undefined;
    let object: SyntaxNode | undefined;

    for (let child: SyntaxNode | null = inner; child !== null; child = child.parent) {
      const parent = child.parent;

      if (kind === undefined && child.name === 'Property') {
        const colon = child.getChild(':');

        if (colon !== null && pos >= colon.to) {
          kind = 'value';
          path.unshift(this.#getPropertyName(state, child));
        }
      } else if (kind === undefined) {
        if (child.name === 'PropertyName' || child.name === 'Object') kind = 'name';
        else if (child.name === 'Array' || (parent?.name === 'Property' && child.name !== 'PropertyName')) {
          kind = 'value';
        }
      }

      if (kind === 'name' && object === undefined && child.name === 'Object') object = child;
      if (parent?.name === 'Property' && child.name !== 'PropertyName') {
        path.unshift(this.#getPropertyName(state, parent));
      } else if (parent?.name === 'Array') {
        path.unshift('*');
      }
    }

    if (kind === undefined || (kind === 'name' && object === undefined)) return undefined;

    const own = inner.name === 'PropertyName' ? this.#getText(state, inner) : undefined;
    const names = (object?.getChildren('Property') ?? [])
      .map((property) => this.#getPropertyName(state, property))
      .filter((name) => name !== own);
    const isValue = ['String', 'PropertyName', 'Number', 'True', 'False', 'Null'].includes(inner.name);

    return { path, kind, node: isValue ? inner : undefined, names };
  }

  #getPropertyName(state: EditorState, property: SyntaxNode): string {
    const name = property.getChild('PropertyName');

    return name === null ? '' : this.#getText(state, name);
  }

  #getText(state: EditorState, node: SyntaxNode): string {
    return state.sliceDoc(node.from, node.to).replace(/^"|"$/g, '');
  }

  #getSchemaAt(path: readonly string[]): JsonObject | undefined {
    let schema = this.#resolveReference(this.#schema);

    for (const name of path) {
      if (schema === undefined) return undefined;

      const properties = asObject(schema['properties']);
      const next =
        name === '*'
          ? schema['items']
          : (properties?.[name] ?? (name === '$schema' ? undefined : schema['additionalProperties']));

      schema = this.#resolveReference(asObject(next ?? undefined));
    }

    return schema;
  }

  /** The schema's own description and default override the referenced definition's. */
  #resolveReference(schema: JsonObject | undefined): JsonObject | undefined {
    if (schema === undefined) return undefined;

    const reference = schema['$ref'];

    if (typeof reference !== 'string') return schema;

    const definitions = asObject(this.#schema['$defs']);
    const definition = asObject(definitions?.[reference.replace('#/$defs/', '')]);

    return { ...this.#resolveReference(definition), ...schema, $ref: null };
  }

  #getChoices(schema: JsonObject): [Json, string | undefined][] {
    const values = Array.isArray(schema['enum']) ? schema['enum'] : [];
    const constants = Array.isArray(schema['oneOf']) ? schema['oneOf'] : [];

    return [
      ...values.map((value): [Json, string | undefined] => [value, undefined]),
      ...constants.flatMap((variant): [Json, string | undefined][] => {
        const option = asObject(variant);

        return option === undefined || !('const' in option) ? [] : [[option['const']!, this.#getDescription(option)]];
      })
    ];
  }

  #getTypes(schema: JsonObject): string[] {
    const type = schema['type'];

    if (Array.isArray(type)) return type.map(String);

    return typeof type === 'string' ? [type] : [];
  }

  #describeType(schema: JsonObject): string {
    const choices = this.#getChoices(schema);

    if (choices.length > 0) return choices.map(([value]) => JSON.stringify(value)).join(' | ');

    return this.#getTypes(schema)
      .map((type) => {
        const items = this.#resolveReference(asObject(schema['items']));

        return type === 'array' && items !== undefined ? `${this.#describeType(items)}[]` : type;
      })
      .join(' | ');
  }

  #getDescription(schema: JsonObject | undefined): string | undefined {
    const description = schema?.['description'];

    return typeof description === 'string' ? description.replaceAll('\n', ' ') : undefined;
  }
}
