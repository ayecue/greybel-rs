import { useContext } from 'preact/hooks';

import { Highlight, type Token } from '../protocol.generated.js';
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
  [Highlight.Type]: 'token type'
};

/** Code highlighted as examples are, which the editor can open; or, with `language`, the plain text of another language. */
export function Example({ tokens, language }: { tokens: readonly Token[]; language?: string }) {
  const { openExample } = useContext(ReferenceContext);

  if (tokens.length === 0) return null;

  return (
    <div class="example-box">
      <pre class="example" data-language={language}>
        <code>
          {tokens.map((token) => (
            <TokenText token={token} />
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

function TokenText({ token }: { token: Token }) {
  const text = token.link === undefined ? token.text : <LinkTo link={token.link}>{token.text}</LinkTo>;
  const className = HIGHLIGHT_CLASSES[token.highlight];

  return className === undefined ? <>{text}</> : <span class={className}>{text}</span>;
}
