import 'vitest';

declare module 'vitest' {
  export interface ProvidedContext {
    rust: string;

    target: string;

    shaders: string;
  }
}
