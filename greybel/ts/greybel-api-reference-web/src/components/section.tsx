import { memo } from 'preact/compat';

import type { Section } from '../protocol.generated.js';
import { Description } from './description.js';
import { Example } from './example.js';
import { TypeName } from './link.js';
import { MemberView } from './member.js';
import { getAnchorId } from './reference-context.js';

export const SectionView = memo(function SectionView({
  section,
  isHidden,
  hiddenMembers
}: {
  section: Section;
  isHidden: boolean;
  hiddenMembers: ReadonlySet<string>;
}) {
  return (
    <section class="section" id={getAnchorId(section.name)} hidden={isHidden}>
      <h2>
        {section.name}
        {section.extends !== undefined && (
          <span class="extends">
            {' extends '}
            <TypeName name={section.extends} />
          </span>
        )}
      </h2>
      <Description blocks={section.description} />
      <Example tokens={section.example} />
      <div class="members">
        {section.members.map((member) => (
          <MemberView
            key={member.name}
            owner={section.name}
            member={member}
            isHidden={hiddenMembers.has(member.name)}
          />
        ))}
      </div>
    </section>
  );
});
