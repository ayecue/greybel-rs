import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';

import oniguruma from 'vscode-oniguruma';
import textmate from 'vscode-textmate';
import { beforeAll, describe, expect, it } from 'vitest';

import { tmLanguage } from '../src/index.js';

const CODE = 'source.src';

let grammar: textmate.IGrammar;

beforeAll(async () => {
  const wasm = await readFile(createRequire(import.meta.url).resolve('vscode-oniguruma/release/onig.wasm'));

  await oniguruma.loadWASM(wasm.buffer as ArrayBuffer);

  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
      createOnigString: (text) => new oniguruma.OnigString(text)
    }),
    loadGrammar: async (scopeName) =>
      scopeName === CODE ? textmate.parseRawGrammar(JSON.stringify(tmLanguage), `${scopeName}.json`) : null
  });
  const loaded = await registry.loadGrammar(CODE);

  if (!loaded) throw new Error('the code grammar did not load');

  grammar = loaded;
});

/** Each non-blank token of `text` with its innermost scope. */
function scopes(text: string): Array<[string, string]> {
  const tokens: Array<[string, string]> = [];
  let state = textmate.INITIAL;

  for (const line of text.split('\n')) {
    const result = grammar.tokenizeLine(line, state);

    for (const token of result.tokens) {
      const value = line.slice(token.startIndex, token.endIndex).trim();

      if (value === '') continue;

      tokens.push([value, token.scopes[token.scopes.length - 1]]);
    }

    state = result.ruleStack;
  }

  return tokens;
}

describe('the code grammar', () => {
  it('should highlight hex, binary and separated number literals', () => {
    const numbers = scopes('x = [0xFF, 0o17, 0b1010, 1_000_000, 1.5e3]').filter(
      ([, scope]) => scope === 'constant.numeric'
    );

    expect(numbers.map(([text]) => text)).toEqual(['0xFF', '0o17', '0b1010', '1_000_000', '1.5e3']);
  });

  it('should highlight let and const before a name at the start of a statement', () => {
    expect(scopes('let count = 0')[0]).toEqual(['let', 'storage.modifier.greybel.greyscript']);
    expect(scopes('  const LIMIT = 10')[0]).toEqual(['const', 'storage.modifier.greybel.greyscript']);
    expect(scopes('let = 5')[0]).not.toEqual(['let', 'storage.modifier.greybel.greyscript']);
    expect(scopes('export run = 1')[0]).toEqual(['export', 'storage.modifier.greybel.greyscript']);
  });

  it('should highlight the text, escapes and expressions of a string template', () => {
    const tokens = scopes('x = #"Hi {count + 1}, {{x}} ""q"""');

    expect(tokens.slice(2)).toEqual([
      ['#"', 'string.interpolated.greyscript'],
      ['Hi', 'string.interpolated.greyscript'],
      ['{', 'punctuation.section.embedded.begin.greyscript'],
      ['count', 'variable.other.greyscript'],
      ['+', 'keyword.operator.greyscript'],
      ['1', 'constant.numeric'],
      ['}', 'punctuation.section.embedded.end.greyscript'],
      [',', 'string.interpolated.greyscript'],
      ['{{', 'constant.character.escape.greyscript'],
      ['x', 'string.interpolated.greyscript'],
      ['}}', 'constant.character.escape.greyscript'],
      ['""', 'constant.character.escape.greyscript'],
      ['q', 'string.interpolated.greyscript'],
      ['""', 'constant.character.escape.greyscript'],
      ['"', 'string.interpolated.greyscript']
    ]);
  });

  it('should highlight enum and end enum', () => {
    expect(scopes('enum Color')[0]).toEqual(['enum', 'keyword.control.block.greybel.greyscript']);
    expect(scopes('end enum')[0]).toEqual(['end enum', 'keyword.control.block.greybel.greyscript']);
    expect(scopes('enum = 1')[0]).not.toEqual(['enum', 'keyword.control.block.greybel.greyscript']);
  });

  it('should highlight assert', () => {
    expect(scopes('assert x > 0, "x"')[0]).toEqual(['assert', 'keyword.control.statement.greybel.greyscript']);
    expect(scopes('assert = 1')[0]).not.toEqual(['assert', 'keyword.control.statement.greybel.greyscript']);
  });

  it('should highlight the arrow of a short function', () => {
    expect(scopes('f = (x) => x')).toContainEqual(['=>', 'storage.type.function.arrow.greybel.greyscript']);
  });

  it('should highlight switch, case and end switch', () => {
    expect(scopes('switch command')[0]).toEqual(['switch', 'keyword.control.block.greybel.greyscript']);
    expect(scopes('  case "ls"')[0]).toEqual(['case', 'keyword.control.block.greybel.greyscript']);
    expect(scopes('end switch')[0]).toEqual(['end switch', 'keyword.control.block.greybel.greyscript']);
  });

  it('should highlight ternary expressions, ?? and optional chains', () => {
    expect(scopes('x = a ? b : c')).toContainEqual(['?', 'keyword.operator.ternary.greybel.greyscript']);
    expect(scopes('x = a ?? b')).toContainEqual(['??', 'keyword.operator.optional.greybel.greyscript']);
    expect(scopes('x = a?.b')).toContainEqual(['?.', 'keyword.operator.optional.greybel.greyscript']);
    expect(scopes('x = c?.5:1')).toContainEqual(['?', 'keyword.operator.ternary.greybel.greyscript']);
  });

  it('should highlight class headers, end class and private', () => {
    expect(scopes('class Dog extends Animal implements Pet')).toEqual(
      expect.arrayContaining([
        ['class', 'keyword.control.block.greybel.greyscript'],
        ['Dog', 'entity.name.type.class.greybel.greyscript'],
        ['extends', 'storage.modifier.greybel.greyscript'],
        ['Animal', 'entity.other.inherited-class.greybel.greyscript'],
        ['implements', 'storage.modifier.greybel.greyscript']
      ])
    );
    expect(scopes('end class')[0]).toEqual(['end class', 'keyword.control.block.greybel.greyscript']);
    expect(scopes('  private items = []')).toContainEqual(['private', 'storage.modifier.greybel.greyscript']);
    expect(scopes('  static private count = 0')).toEqual(
      expect.arrayContaining([
        ['static', 'storage.modifier.greybel.greyscript'],
        ['private', 'storage.modifier.greybel.greyscript']
      ])
    );
    expect(scopes('class = 1')[0]).not.toEqual(['class', 'keyword.control.block.greybel.greyscript']);
  });

  it('should highlight types written in code', () => {
    expect(scopes('f = function<T>(ip: string, port: number = 22): map|null')).toEqual(
      expect.arrayContaining([
        ['T', 'entity.name.type.greyscript'],
        ['ip', 'variable.parameter.greyscript'],
        ['string', 'support.type.primitive.greyscript'],
        ['port', 'variable.parameter.greyscript'],
        ['map', 'support.type.primitive.greyscript'],
        ['|', 'keyword.operator.type.greyscript'],
        ['null', 'support.type.primitive.greyscript']
      ])
    );
    expect(scopes('let port: Port = 22')).toEqual(
      expect.arrayContaining([
        ['let', 'storage.modifier.greybel.greyscript'],
        ['Port', 'entity.name.type.greyscript']
      ])
    );
    expect(scopes('s = get_shell as shell')).toEqual(
      expect.arrayContaining([
        ['as', 'keyword.operator.type.greyscript'],
        ['shell', 'entity.name.type.greyscript']
      ])
    );
    expect(scopes('#import {a as b} from "x"')).not.toContainEqual(['b', 'entity.name.type.greyscript']);
  });

  it('should highlight type statements', () => {
    expect(scopes('type Id = string|number')).toEqual(
      expect.arrayContaining([
        ['type', 'storage.type.greybel.greyscript'],
        ['Id', 'entity.name.type.greyscript'],
        ['number', 'support.type.primitive.greyscript']
      ])
    );
    expect(scopes('type Repo extends map\n  readonly id?: Id\n  find(key: Id): map\nend type')).toEqual(
      expect.arrayContaining([
        ['extends', 'storage.modifier.greybel.greyscript'],
        ['readonly', 'storage.modifier.greybel.greyscript'],
        ['id', 'variable.other.property.greyscript'],
        ['find', 'entity.name.function.greyscript'],
        ['key', 'variable.parameter.greyscript'],
        ['end type', 'storage.type.greybel.greyscript']
      ])
    );
    expect(scopes('type = 1')[0]).not.toEqual(['type', 'storage.type.greybel.greyscript']);
  });

  it('should highlight const enum', () => {
    expect(scopes('const enum Port')).toEqual(
      expect.arrayContaining([
        ['const', 'storage.modifier.greybel.greyscript'],
        ['enum', 'keyword.control.block.greybel.greyscript']
      ])
    );
  });

  it('should highlight as in destructuring and imports as a keyword', () => {
    const tokens = scopes('#import {parse, format as fmt} from "lib.src"\n{port as p} = config');

    expect(tokens.filter(([text]) => text === 'as')).toEqual([
      ['as', 'keyword.control.as.greybel.greyscript'],
      ['as', 'keyword.control.as.greybel.greyscript']
    ]);
  });

  it('should highlight spread, non-null and case is', () => {
    const tokens = scopes('l = [...a]\ny = value!\nz = a != b\ncase is Dog');

    expect(tokens).toContainEqual(['...', 'keyword.operator.spread.greybel.greyscript']);
    expect(tokens).toContainEqual(['!', 'keyword.operator.non-null.greybel.greyscript']);
    expect(tokens).toContainEqual(['!=', 'keyword.operator.greyscript']);
    expect(tokens).toContainEqual(['is', 'keyword.operator.type.greyscript']);
  });

  it('should highlight the parameters of a short function', () => {
    const tokens = scopes('g = (x: number, y = 2) => x * 2');

    expect(tokens.slice(2, 10)).toEqual([
      ['(', 'source.src'],
      ['x', 'variable.parameter.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['number', 'support.type.primitive.greyscript'],
      [',', 'source.src'],
      ['y', 'variable.parameter.greyscript'],
      ['=', 'source.src'],
      ['2', 'constant.numeric']
    ]);
  });

  it('should highlight class methods but not calls in their bodies', () => {
    const tokens = scopes(
      'class A\n  private run(x)\n    cb = function(y)\n      print(y)\n    end function\n    print(x)\n  end function\nend class\nprint(2)'
    );

    expect(tokens.filter(([, scope]) => scope === 'entity.name.function.greyscript')).toEqual([
      ['run', 'entity.name.function.greyscript']
    ]);
    expect(tokens).toContainEqual(['private', 'storage.modifier.greybel.greyscript']);
    expect(tokens.filter(([text]) => text === 'x')[0]).toEqual(['x', 'variable.parameter.greyscript']);
    expect(tokens.filter(([text]) => text === 'print').map(([, scope]) => scope)).toEqual([
      'support.function.greyscript',
      'support.function.greyscript',
      'support.function.greyscript'
    ]);
    expect(tokens).toContainEqual(['end class', 'keyword.control.block.greybel.greyscript']);
  });
});
