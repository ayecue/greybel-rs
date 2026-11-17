import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';

import oniguruma from 'vscode-oniguruma';
import textmate from 'vscode-textmate';
import { beforeAll, describe, expect, it } from 'vitest';

import { signatureTmLanguage, tmLanguage } from '../src/index.js';

const SIGNATURE = 'source.greyscript-signature';

let grammar: textmate.IGrammar;

beforeAll(async () => {
  const wasm = await readFile(createRequire(import.meta.url).resolve('vscode-oniguruma/release/onig.wasm'));

  await oniguruma.loadWASM(wasm.buffer as ArrayBuffer);

  const grammars: Record<string, unknown> = {
    [SIGNATURE]: signatureTmLanguage,
    'source.src': tmLanguage
  };
  const registry = new textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
      createOnigString: (text) => new oniguruma.OnigString(text)
    }),
    loadGrammar: async (scopeName) => textmate.parseRawGrammar(JSON.stringify(grammars[scopeName]), `${scopeName}.json`)
  });
  const loaded = await registry.loadGrammar(SIGNATURE);

  if (!loaded) throw new Error('the signature grammar did not load');

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

describe('the signature grammar for hovers', () => {
  it('should highlight the name, parameters, defaults and return type of a function', () => {
    const tokens = scopes('get_shell: function(user: string = "", password: string = ""): shell or null or string');

    expect(tokens).toEqual([
      ['get_shell', 'entity.name.function.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['function', 'support.type.primitive.greyscript'],
      ['(', 'punctuation.definition.parameters.begin.greyscript'],
      ['user', 'variable.parameter.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['string', 'support.type.primitive.greyscript'],
      ['=', 'keyword.operator.assignment.greyscript'],
      ['"', 'string.quoted.double.greyscript'],
      ['"', 'string.quoted.double.greyscript'],
      [',', 'punctuation.separator.comma.greyscript'],
      ['password', 'variable.parameter.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['string', 'support.type.primitive.greyscript'],
      ['=', 'keyword.operator.assignment.greyscript'],
      ['"', 'string.quoted.double.greyscript'],
      ['"', 'string.quoted.double.greyscript'],
      [')', 'punctuation.definition.parameters.end.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['shell', 'entity.name.type.greyscript'],
      ['or', 'keyword.operator.expression.or.greyscript'],
      ['null', 'support.type.primitive.greyscript'],
      ['or', 'keyword.operator.expression.or.greyscript'],
      ['string', 'support.type.primitive.greyscript']
    ]);
  });

  it('should highlight source defaults as code up to the next parameter', () => {
    const tokens = scopes('fill: function(items: list<number> = [1, 2], cb: any = @print, size: number?): list<any>');

    expect(tokens).toEqual([
      ['fill', 'entity.name.function.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['function', 'support.type.primitive.greyscript'],
      ['(', 'punctuation.definition.parameters.begin.greyscript'],
      ['items', 'variable.parameter.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['list', 'support.type.primitive.greyscript'],
      ['<', 'punctuation.definition.typeparameters.begin.greyscript'],
      ['number', 'support.type.primitive.greyscript'],
      ['>', 'punctuation.definition.typeparameters.end.greyscript'],
      ['=', 'keyword.operator.assignment.greyscript'],
      ['[', SIGNATURE],
      ['1', 'constant.numeric'],
      [',', SIGNATURE],
      ['2', 'constant.numeric'],
      [']', SIGNATURE],
      [',', 'punctuation.separator.comma.greyscript'],
      ['cb', 'variable.parameter.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['any', 'support.type.primitive.greyscript'],
      ['=', 'keyword.operator.assignment.greyscript'],
      ['@', SIGNATURE],
      ['print', 'support.function.greyscript'],
      [',', 'punctuation.separator.comma.greyscript'],
      ['size', 'variable.parameter.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['number', 'support.type.primitive.greyscript'],
      ['?', 'keyword.operator.optional.greyscript'],
      [')', 'punctuation.definition.parameters.end.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['list', 'support.type.primitive.greyscript'],
      ['<', 'punctuation.definition.typeparameters.begin.greyscript'],
      ['any', 'support.type.primitive.greyscript'],
      ['>', 'punctuation.definition.typeparameters.end.greyscript']
    ]);
  });

  it('should highlight the declared type and the entries of a map', () => {
    const tokens = scopes(
      [
        'x: map (declared: map or null) {',
        '  "count": number,',
        '  "say ""hi""": function(): null,',
        '  1: list<map<string,number>>,',
        '  [any]: any',
        '  ... 3 more',
        '}'
      ].join('\n')
    );

    expect(tokens).toEqual([
      ['x', 'variable.other.readwrite.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['map', 'support.type.primitive.greyscript'],
      ['(', 'punctuation.section.parens.begin.greyscript'],
      ['declared', 'storage.modifier.declared.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['map', 'support.type.primitive.greyscript'],
      ['or', 'keyword.operator.expression.or.greyscript'],
      ['null', 'support.type.primitive.greyscript'],
      [')', 'punctuation.section.parens.end.greyscript'],
      ['{', 'punctuation.definition.dictionary.begin.greyscript'],
      ['"count"', 'string.quoted.double.greyscript'],
      [':', 'punctuation.separator.key-value.greyscript'],
      ['number', 'support.type.primitive.greyscript'],
      [',', 'punctuation.separator.comma.greyscript'],
      ['"say ""hi"""', 'string.quoted.double.greyscript'],
      [':', 'punctuation.separator.key-value.greyscript'],
      ['function', 'support.type.primitive.greyscript'],
      ['(', 'punctuation.definition.parameters.begin.greyscript'],
      [')', 'punctuation.definition.parameters.end.greyscript'],
      [':', 'keyword.operator.type.annotation.greyscript'],
      ['null', 'support.type.primitive.greyscript'],
      [',', 'punctuation.separator.comma.greyscript'],
      ['1', 'constant.numeric.greyscript'],
      [':', 'punctuation.separator.key-value.greyscript'],
      ['list', 'support.type.primitive.greyscript'],
      ['<', 'punctuation.definition.typeparameters.begin.greyscript'],
      ['map', 'support.type.primitive.greyscript'],
      ['<', 'punctuation.definition.typeparameters.begin.greyscript'],
      ['string', 'support.type.primitive.greyscript'],
      [',', 'punctuation.separator.comma.greyscript'],
      ['number', 'support.type.primitive.greyscript'],
      ['>', 'punctuation.definition.typeparameters.end.greyscript'],
      ['>', 'punctuation.definition.typeparameters.end.greyscript'],
      [',', 'punctuation.separator.comma.greyscript'],
      ['[', 'punctuation.definition.index.begin.greyscript'],
      ['any', 'support.type.primitive.greyscript'],
      [']', 'punctuation.definition.index.end.greyscript'],
      [':', 'punctuation.separator.key-value.greyscript'],
      ['any', 'support.type.primitive.greyscript'],
      ['... 3 more', 'comment.line.more.greyscript'],
      ['}', 'punctuation.definition.dictionary.end.greyscript']
    ]);
  });

  it('should highlight a type without a name', () => {
    const tokens = scopes('Point<number> or funcRef');

    expect(tokens).toEqual([
      ['Point', 'entity.name.type.greyscript'],
      ['<', 'punctuation.definition.typeparameters.begin.greyscript'],
      ['number', 'support.type.primitive.greyscript'],
      ['>', 'punctuation.definition.typeparameters.end.greyscript'],
      ['or', 'keyword.operator.expression.or.greyscript'],
      ['funcRef', 'support.type.primitive.greyscript']
    ]);
  });

  it('should highlight the return type of a type guard', () => {
    const tokens = scopes('isDog: function(value: any): value is Dog');

    expect(tokens.slice(-3)).toEqual([
      ['value', 'variable.parameter.greyscript'],
      ['is', 'keyword.operator.expression.is.greyscript'],
      ['Dog', 'entity.name.type.greyscript']
    ]);
  });
});
