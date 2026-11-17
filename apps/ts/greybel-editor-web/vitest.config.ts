import { mergeConfig } from 'vitest/config';

import { vitest } from '../config.ts';

// Test files run in parallel, so polls wait up to 5 s.
export default mergeConfig(vitest, { test: { testTimeout: 30_000, expect: { poll: { timeout: 5_000 } } } });
