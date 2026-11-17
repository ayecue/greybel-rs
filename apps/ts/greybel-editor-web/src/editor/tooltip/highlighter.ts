import { StreamLanguage, type Language } from '@codemirror/language';

import { CodeTokenizer } from './code-tokenizer.js';
import { SIGNATURE_LANGUAGE, SignatureTokenizer } from './signature-tokenizer.js';

const CODE_LANGUAGES = ['greyscript', 'miniscript', 'greybel'];

/** The languages of the code blocks in the language server's hovers, completions and signature help. */
export class TooltipHighlighter {
  readonly #code: Language;
  readonly #signature: Language;

  constructor() {
    const code = new CodeTokenizer();

    this.#code = StreamLanguage.define(code);
    this.#signature = StreamLanguage.define(new SignatureTokenizer(code));
  }

  /** Null for languages tooltips show as plain text. */
  findLanguage(name: string): Language | null {
    if (name === SIGNATURE_LANGUAGE) return this.#signature;

    return CODE_LANGUAGES.includes(name) ? this.#code : null;
  }
}
