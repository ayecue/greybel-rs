import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => ({
  base: './',
  build: {
    outDir: mode === 'test' ? 'dist-test' : 'dist',
    target: 'es2022',
    sourcemap: true
  }
}));
