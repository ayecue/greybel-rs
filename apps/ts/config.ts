import { vitestConfig } from 'presets/vitest';

export const folders = {
  rust: new URL('../../greybel/rs/', import.meta.url),
  target: new URL('../../target/', import.meta.url),
  shaders: new URL('../../text-mesh/shaders/', import.meta.url)
};

export const vitest = vitestConfig(folders);
