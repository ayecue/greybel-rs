import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => ({
  base: './',
  envPrefix: ['VITE_', 'EDITOR_SERVICE_URL'],
  build: {
    outDir: mode === 'test' ? 'dist-test' : 'dist',
    target: 'es2022',
    sourcemap: true
  },
  worker: {
    format: 'es'
  }
}));
