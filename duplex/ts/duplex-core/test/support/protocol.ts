import { readdirSync } from 'node:fs';
import { join } from 'node:path';

import { inject } from 'vitest';

export function versions(): [number, number][] {
  return readdirSync(join(inject('duplex'), 'core'))
    .map((file) => /^(\d+)\.(\d+)\.protocol$/.exec(file))
    .filter((match) => match !== null)
    .map((match): [number, number] => [Number(match[1]), Number(match[2])])
    .sort(([major, minor], [otherMajor, otherMinor]) => major - otherMajor || minor - otherMinor);
}
