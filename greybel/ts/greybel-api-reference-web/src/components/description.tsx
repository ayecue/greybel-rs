import { useContext } from 'preact/hooks';

import { Block, Link, Style, type Span, type TableCell } from '../protocol.generated.js';
import { Example } from './example.js';
import { LinkTo } from './link.js';
import { ReferenceContext } from './reference-context.js';

/** `blocks` as the page shows them; `headingIds` gives headings the ids links scroll to, with a link to copy. */
export function Description({
  blocks,
  headingIds
}: {
  blocks: readonly Block[];
  headingIds?: ReadonlyMap<Block, string>;
}) {
  return (
    <div class="description">
      {blocks.map((block) => (
        <BlockView block={block} id={headingIds?.get(block)} />
      ))}
    </div>
  );
}

function BlockView({ block, id }: { block: Block; id?: string }) {
  if (block instanceof Block.List) {
    return (
      <ul>
        {block.items.map((item) => (
          <li>
            <Spans spans={item.spans} />
          </li>
        ))}
      </ul>
    );
  }

  if (block instanceof Block.Heading) {
    // The page's title is its only h1, so Markdown's levels start at h2.
    const Heading = `h${Math.min(block.level + 1, 6)}` as 'h2';

    return (
      <Heading id={id}>
        <Spans spans={block.spans} />
        {id !== undefined && <HeadingLink anchor={id} />}
      </Heading>
    );
  }

  if (block instanceof Block.Code) return <Example tokens={block.tokens} language={block.language} />;

  if (block instanceof Block.Table) {
    return (
      <div class="table">
        <table>
          <thead>
            <tr>
              <Cells cells={block.head} Cell="th" />
            </tr>
          </thead>
          <tbody>
            {block.rows.map((row) => (
              <tr>
                <Cells cells={row.cells} Cell="td" />
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  return (
    <p>
      <Spans spans={(block as Block.Paragraph).spans} />
    </p>
  );
}

/**
 * The `#` after a heading (drawn by the style sheet, so it isn't part of the heading's text): it scrolls to the
 * heading, and copies the link to it where the page can link.
 */
function HeadingLink({ anchor }: { anchor: string }) {
  const { copyLink } = useContext(ReferenceContext);
  const link = new Link.Page({ anchor });

  return (
    <LinkTo
      link={link}
      className="heading-link"
      title="Copy the link to this part"
      onFollow={() => copyLink?.(link)}
      children={null}
    />
  );
}

function Cells({ cells, Cell }: { cells: readonly TableCell[]; Cell: 'th' | 'td' }) {
  return (
    <>
      {cells.map((cell) => (
        <Cell>
          <Spans spans={cell.spans} />
        </Cell>
      ))}
    </>
  );
}

function Spans({ spans }: { spans: readonly Span[] }) {
  return (
    <>
      {spans.map((span) => (
        <SpanText span={span} />
      ))}
    </>
  );
}

function SpanText({ span }: { span: Span }) {
  const content = <StyledText span={span} />;

  return span.link === undefined ? content : <LinkTo link={span.link}>{content}</LinkTo>;
}

function StyledText({ span: { style, text } }: { span: Span }) {
  switch (style) {
    case Style.Code:
      return <code>{text}</code>;
    case Style.Emphasis:
      return <em>{text}</em>;
    case Style.Strong:
      return <strong>{text}</strong>;
    default:
      return <>{text}</>;
  }
}
