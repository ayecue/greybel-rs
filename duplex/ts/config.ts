import { vitestConfig } from 'presets/vitest';

export const folders = {
  duplex: new URL('../', import.meta.url),
  rust: new URL('../rs/', import.meta.url),
  target: new URL('../../target/', import.meta.url)
};

export const vitest = vitestConfig(folders);
