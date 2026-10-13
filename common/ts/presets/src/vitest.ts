import { fileURLToPath } from 'node:url';
import { defaultClientConditions, defaultServerConditions } from 'vite';
import { defineConfig, type ViteUserConfig } from 'vitest/config';

export type FolderTable = Readonly<Record<string, URL>>;

export function vitestConfig(folders: FolderTable): ViteUserConfig {
  return defineConfig({
    resolve: { conditions: ['source', ...defaultClientConditions] },
    ssr: { resolve: { conditions: ['source', ...defaultServerConditions] } },
    test: {
      provide: Object.fromEntries(Object.entries(folders).map(([name, url]) => [name, fileURLToPath(url)]))
    }
  });
}
