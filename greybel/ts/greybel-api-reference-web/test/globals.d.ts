import type * as bundle from '../src/index.js';
import type * as page from './support/page.js';

declare global {
  const greybelApiReferenceWeb: typeof bundle;

  const referencePage: typeof page;
}
