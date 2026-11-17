import type { StreamParser, StringStream } from '@codemirror/language';

const KEYWORDS = new Set([
  'if',
  'then',
  'else',
  'end',
  'while',
  'for',
  'in',
  'function',
  'return',
  'break',
  'continue',
  'and',
  'or',
  'not',
  'new',
  'isa'
]);
const CONSTANTS = new Set(['null', 'true', 'false', 'self', 'super', 'globals', 'locals', 'outer']);

export const NAME = /^[\p{L}_][\w]*/u;

/** Tokenizes GreyScript code examples. Its members are arrow functions since `StreamLanguage` calls them unbound. */
export class CodeTokenizer implements StreamParser<null> {
  readonly name = 'greyscript';

  readonly startState = (): null => null;

  readonly token = (stream: StringStream): string | null => {
    if (stream.eatSpace()) return null;
    if (stream.match('//')) {
      stream.skipToEnd();

      return 'comment';
    }

    if (stream.match(/^#[a-z]+/)) return 'keyword';

    const literal = this.readLiteral(stream);

    if (literal !== undefined) return literal;

    const name = stream.match(NAME) as RegExpMatchArray | null;

    if (name !== null) {
      const word = name[0];

      if (KEYWORDS.has(word)) return 'keyword';
      if (CONSTANTS.has(word)) return 'atom';

      return stream.match(/^\s*\(/, false) ? 'variableName.function' : 'variableName';
    }

    stream.next();

    return /[()[\]{},.:]/.test(stream.current()) ? 'punctuation' : 'operator';
  };

  readonly readLiteral = (stream: StringStream): string | undefined => {
    if (stream.match(/^"(?:[^"]|"")*"?/)) return 'string';
    if (stream.match(/^(?:\d+(?:\.\d+)?|\.\d+)(?:[eE][-+]?\d+)?/)) return 'number';

    return undefined;
  };
}
