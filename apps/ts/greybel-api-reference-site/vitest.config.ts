import { mergeConfig } from 'vitest/config';

import { vitest } from '../config.ts';

// Each test drives Chromium with the reference's worker.
export default mergeConfig(vitest, { test: { testTimeout: 30_000 } });
