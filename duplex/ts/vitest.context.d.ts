import 'vitest';

declare module 'vitest' {
  export interface ProvidedContext {
    duplex: string;

    rust: string;

    target: string;
  }
}
