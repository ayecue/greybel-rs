import type { StreamParser, StringStream } from '@codemirror/language';

import { CodeTokenizer, NAME } from './code-tokenizer.js';

/** Same as greybel-lsp's `Markup::SIGNATURE_LANGUAGE`. */
export const SIGNATURE_LANGUAGE = 'greyscript-signature';

const PRIMITIVE_TYPES = new Set([
  'any',
  'unknown',
  'never',
  'null',
  'number',
  'string',
  'list',
  'map',
  'funcRef',
  'function'
]);

export interface SignatureState {
  /** Innermost last; `parameters` marks a parameter list. */
  brackets: string[];
  /** Bracket depth where the current default value started. */
  defaultDepth: number | undefined;
  isParameterListNext: boolean;
  isParameterNext: boolean;
}

/**
 * Tokenizes hover signatures as greyscript-textmate's signature grammar does. Its members are arrow functions since
 * `StreamLanguage` calls them unbound.
 */
export class SignatureTokenizer implements StreamParser<SignatureState> {
  readonly name = SIGNATURE_LANGUAGE;

  readonly #code: CodeTokenizer;

  constructor(code: CodeTokenizer) {
    this.#code = code;
  }

  readonly startState = (): SignatureState => ({
    brackets: [],
    defaultDepth: undefined,
    isParameterListNext: false,
    isParameterNext: false
  });

  readonly copyState = (state: SignatureState): SignatureState => ({ ...state, brackets: [...state.brackets] });

  readonly token = (stream: StringStream, state: SignatureState): string | null => {
    if (stream.sol() && stream.match(/^\s+\.\.\.\s+\d+\s+more$/)) return 'comment';
    if (stream.eatSpace()) return null;
    if (state.defaultDepth !== undefined) return this.#readDefaultToken(stream, state);

    if (stream.column() === 0 && stream.match(/^type(?=\s)/)) return 'keyword';
    if (stream.column() === 0 && stream.match(NAME)) {
      return stream.match(/^:\s+function\b/, false) ? 'variableName.function.definition' : 'variableName.definition';
    }

    const literal = this.#code.readLiteral(stream);

    if (literal !== undefined) return literal;
    if (stream.match(/^\(declared(?=:)/)) {
      state.brackets.push('(');

      return 'comment';
    }

    const isParameterList = state.brackets.at(-1) === 'parameters';
    const name = stream.match(NAME) as RegExpMatchArray | null;

    if (name !== null) return this.#classifyName(stream, state, name[0]);

    const char = stream.next() ?? '';
    const isParameterListStart = char === '(' && state.isParameterListNext;

    state.isParameterListNext = false;
    state.isParameterNext = isParameterListStart || (char === ',' && isParameterList);
    if (char === '=' && isParameterList) {
      state.defaultDepth = state.brackets.length;

      return 'operator';
    }

    if (isParameterListStart) state.brackets.push('parameters');
    else if ('([{<'.includes(char)) state.brackets.push(char);
    if (')]}>'.includes(char)) state.brackets.pop();

    return char === '?' ? 'operator' : 'punctuation';
  };

  #classifyName(stream: StringStream, state: SignatureState, word: string): string {
    const isParameter = state.isParameterNext;

    state.isParameterNext = false;
    state.isParameterListNext = word === 'function' && stream.match(/^\(/, false) !== null;
    if (word === 'or' || word === 'is') return 'operatorKeyword';
    if (isParameter || stream.match(/^\s+is\b/, false)) return 'variableName.special';

    return PRIMITIVE_TYPES.has(word) ? 'typeName.standard' : 'typeName';
  }

  #readDefaultToken(stream: StringStream, state: SignatureState): string | null {
    const depth = state.defaultDepth ?? 0;

    if (state.brackets.length === depth && stream.match(/^[,)]/, false)) {
      state.defaultDepth = undefined;

      return this.token(stream, state);
    }

    if (stream.match(/^[([{]/)) {
      state.brackets.push(stream.current());

      return 'punctuation';
    }

    if (state.brackets.length > depth && stream.match(/^[)\]}]/)) {
      state.brackets.pop();

      return 'punctuation';
    }

    return this.#code.token(stream);
  }
}
