import { describe, expect, it } from 'vitest';

import { CORE } from '../../src/index.js';
import { versions } from '../support/protocol.js';

describe('CORE', () => {
  it('should be the newest version of the core', () => {
    const newest = versions().at(-1);

    expect(newest).toEqual([CORE.major, CORE.minor]);
  });
});
