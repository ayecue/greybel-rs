import { mergeConfig } from 'vitest/config';

import { vitest } from '../config.ts';

// Each test drives Chromium with a language server, a runtime and a terminal in workers.
export default mergeConfig(vitest, { test: { testTimeout: 30_000 } });
