import type { ComponentChildren } from 'preact';
import { useContext } from 'preact/hooks';

import { Link } from '../protocol.generated.js';
import { ReferenceContext, linkedId } from './reference-context.js';

export function LinkTo({
  link,
  className,
  title,
  onFollow,
  children
}: {
  link: Link;
  className?: string;
  title?: string;
  /** Called when the link is followed on the page, after it scrolls there. */
  onFollow?: () => void;
  children: ComponentChildren;
}) {
  const { reveal } = useContext(ReferenceContext);

  if (link instanceof Link.Web) {
    return (
      <a class={className} href={link.url} target="_blank" rel="noopener noreferrer">
        {children}
      </a>
    );
  }

  return (
    <a
      class={className}
      title={title}
      href={`#${linkedId(link) ?? ''}`}
      onClick={(event) => {
        event.preventDefault();
        reveal(link);
        onFollow?.();
      }}
    >
      {children}
    </a>
  );
}

export function TypeName({ name }: { name: string }) {
  const { hasSection } = useContext(ReferenceContext);

  return hasSection(name) ? (
    <LinkTo link={new Link.Type({ name })} className="type">
      {name}
    </LinkTo>
  ) : (
    <span class="type">{name}</span>
  );
}
