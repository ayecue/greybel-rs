import { highlightCode, tagHighlighter, tags } from '@lezer/highlight';
import { describe, expect, it } from 'vitest';

import { TooltipHighlighter } from '../src/editor/tooltip/highlighter.js';

const tooltips = new TooltipHighlighter();

const highlighter = tagHighlighter([
  { tag: tags.definition(tags.variableName), class: 'definition' },
  { tag: tags.function(tags.definition(tags.variableName)), class: 'function-definition' },
  { tag: tags.function(tags.variableName), class: 'function' },
  { tag: tags.special(tags.variableName), class: 'parameter' },
  { tag: tags.variableName, class: 'variable' },
  { tag: tags.standard(tags.typeName), class: 'primitive' },
  { tag: tags.typeName, class: 'type' },
  { tag: [tags.keyword, tags.operatorKeyword], class: 'keyword' },
  { tag: tags.atom, class: 'atom' },
  { tag: tags.string, class: 'string' },
  { tag: tags.number, class: 'number' },
  { tag: tags.comment, class: 'comment' }
]);

function getTokens(name: string, text: string): string[] {
  const language = tooltips.findLanguage(name);

  if (language === null) throw new Error(`no language ${name}`);

  const result: string[] = [];

  highlightCode(
    text,
    language.parser.parse(text),
    highlighter,
    (part, classes) => {
      if (classes === '') return;
      result.push(`${part}:${classes}`);
    },
    () => undefined
  );

  return result;
}

describe('the highlighting of tooltips', () => {
  it('should highlight a function signature with its parameters, defaults and return types', () => {
    expect(
      getTokens('greyscript-signature', 'open: function(ip: string, port?: number = 22, f = @print): shell or null')
    ).toEqual([
      'open:function-definition',
      'function:primitive',
      'ip:parameter',
      'string:primitive',
      'port:parameter',
      'number:primitive',
      '22:number',
      'f:parameter',
      'print:variable',
      'shell:type',
      'or:keyword',
      'null:primitive'
    ]);
  });

  it('should highlight a variable, its declared type, map entries and a type guard', () => {
    expect(getTokens('greyscript-signature', 'x: list<number> (declared: string or Point)')).toEqual([
      'x:definition',
      'list:primitive',
      'number:primitive',
      '(declared:comment',
      'string:primitive',
      'or:keyword',
      'Point:type'
    ]);
    expect(getTokens('greyscript-signature', 'm: map {\n  "name": string,\n  ... 3 more\n}')).toEqual([
      'm:definition',
      'map:primitive',
      '"name":string',
      'string:primitive',
      '  ... 3 more:comment'
    ]);
    expect(getTokens('greyscript-signature', 'f: function(v: any): v is string')).toContain('v:parameter');
    expect(getTokens('greyscript-signature', 'f: function(v: any): v is string')).toContain('is:keyword');
  });

  it('should highlight GreyScript code examples', () => {
    expect(getTokens('greyscript', 'if x then print("a") // b')).toEqual([
      'if:keyword',
      'x:variable',
      'then:keyword',
      'print:function',
      '"a":string',
      '// b:comment'
    ]);
    expect(tooltips.findLanguage('json')).toBeNull();
  });
});
