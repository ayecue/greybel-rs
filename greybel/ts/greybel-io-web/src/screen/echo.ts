import { Echo } from '../protocol.generated.js';

const MASK_CHARACTER = '*';

/** Display mode for `line` during input: unchanged, one `*` per character, or hidden. */
export function formatEcho(line: string, echo: Echo): string {
  switch (echo) {
    case Echo.Visible:
      return line;
    case Echo.Masked:
      return MASK_CHARACTER.repeat([...line].length);
    case Echo.Hidden:
      return '';
  }
}
