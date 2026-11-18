import { useContext } from 'preact/hooks';

import { Highlight, type Token } from '../protocol.generated.js';
import { Diagram } from './diagram.js';
import { LinkTo } from './link.js';
import { ReferenceContext } from './reference-context.js';

const HIGHLIGHT_CLASSES: Record<Highlight, string | undefined> = {
  [Highlight.Plain]: undefined,
  [Highlight.Keyword]: 'token keyword',
  [Highlight.String]: 'token string',
  [Highlight.Number]: 'token number',
  [Highlight.Constant]: 'token constant',
  [Highlight.Comment]: 'token comment',
  [Highlight.Name]: 'token name',
  [Highlight.Operator]: 'token operator',
  [Highlight.Type]: 'token type',
  [Highlight.Library]: 'token library'
};

/** Bracket colors cycle through this many classes by nesting depth, as in Grey Hack's code editor. */
const BRACKET_DEPTHS = 6;

const BRACKET_NAMES: Partial<Record<string, string>> = { '(': 'paren', ')': 'paren', '[': 'square', ']': 'square' };

/** Gives brackets a class by nesting depth, `paren-<1..6>` or `square-<1..6>`, and colons `colon`. */
class BracketDepth {
  readonly #depths = new Map<string, number>();

  getClass(text: string): string | undefined {
    if (text === ':') return 'colon';

    const name = BRACKET_NAMES[text];

    if (name === undefined) return undefined;

    const isOpening = text === '(' || text === '[';
    const depth = (this.#depths.get(name) ?? 0) + (isOpening ? 1 : 0);

    this.#depths.set(name, isOpening ? depth : depth - 1);

    return depth < 1 ? 'unmatched' : `${name}-${((depth - 1) % BRACKET_DEPTHS) + 1}`;
  }
}

/** Code highlighted as examples are, which the editor can open; or, with `language`, the plain text of another language. */
export function Example({ tokens, language }: { tokens: readonly Token[]; language?: string }) {
  const { openExample } = useContext(ReferenceContext);

  if (tokens.length === 0) return null;
  if (language === 'mermaid') return <Diagram source={tokens.map((token) => token.text).join('')} />;

  const brackets = new BracketDepth();

  return (
    <div class="example-box">
      <pre class="example" data-language={language}>
        <code>
          {tokens.map((token) => (
            <TokenText
              token={token}
              bracket={token.highlight === Highlight.Operator ? brackets.getClass(token.text) : undefined}
            />
          ))}
        </code>
      </pre>
      {openExample !== undefined && language === undefined && (
        <button type="button" class="action" onClick={() => openExample(tokens.map((token) => token.text).join(''))}>
          Open in editor
        </button>
      )}
    </div>
  );
}

function TokenText({ token, bracket }: { token: Token; bracket: string | undefined }) {
  const text = token.link === undefined ? token.text : <LinkTo link={token.link}>{token.text}</LinkTo>;
  const highlight = HIGHLIGHT_CLASSES[token.highlight];
  const className = bracket === undefined ? highlight : `${highlight} ${bracket}`;

  return className === undefined ? <>{text}</> : <span class={className}>{text}</span>;
}
