import { createContext } from 'preact';

import { Link } from '../protocol.generated.js';

export interface ReferenceContextValue {
  readonly reveal: (link: Link) => void;

  readonly hasSection: (name: string) => boolean;

  /** Copies the documentation URL at `link` for a member or guide section. */
  readonly copyLink?: (link: Link) => void;

  readonly openExample?: (code: string) => void;
}

export const ReferenceContext = createContext<ReferenceContextValue>({ reveal: () => {}, hasSection: () => false });

export function getAnchorId(name: string, member?: string): string {
  return member === undefined ? `api-${name}` : `api-${name}.${member}`;
}

export function getLinkedId(link: Link): string | undefined {
  if (link instanceof Link.Member) return getAnchorId(link.owner, link.name);
  if (link instanceof Link.Type) return getAnchorId(link.name);
  if (link instanceof Link.Page) return link.anchor;

  return undefined;
}
