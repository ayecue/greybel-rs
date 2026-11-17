import type * as bundle from '../src/index.js';
import type * as client from './support/client.js';

declare global {
  const greybelApiReferenceWeb: typeof bundle;

  const referencePage: typeof client;
}
