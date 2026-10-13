import { memo } from 'preact/compat';
import { useContext } from 'preact/hooks';

import { Link, MemberKind, type Member } from '../protocol.generated.js';
import { Description } from './description.js';
import { Example } from './example.js';
import { ReferenceContext, anchorId } from './reference-context.js';
import { LiteralValue, Signature } from './signature.js';

export const MemberView = memo(function MemberView({
  owner,
  member,
  isHidden
}: {
  owner: string;
  member: Member;
  isHidden: boolean;
}) {
  const { copyLink } = useContext(ReferenceContext);
  const { kind } = member;

  return (
    <article class="member" id={anchorId(owner, member.name)} hidden={isHidden}>
      <header>
        <h3>
          <span class="name">{member.name}</span>
          <Signature kind={kind} />
        </h3>
        <span class="tags">
          {member.tags.map((tag) => (
            <span class="tag">{tag}</span>
          ))}
        </span>
        {copyLink !== undefined && (
          <button type="button" class="action" onClick={() => copyLink(new Link.Member({ owner, name: member.name }))}>
            Copy link
          </button>
        )}
      </header>
      <Description blocks={member.description} />
      {kind instanceof MemberKind.Function && kind.returnVariations.length > 0 && (
        <details class="variations">
          <summary>Known return values</summary>
          <ul>
            {kind.returnVariations.map((variation) => (
              <li>
                <LiteralValue literal={variation} />
              </li>
            ))}
          </ul>
        </details>
      )}
      <Example tokens={member.example} />
    </article>
  );
});
