import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest';

import { session } from './support/browser.js';
import { BUILD_TIMEOUT } from './support/example.js';

beforeAll(() => session.start(), BUILD_TIMEOUT);

afterEach(() => session.stopCalculators());

afterAll(() => session.close());

describe('the Rust calculator over a WebSocket from a page in Chromium', () => {
  it('should serve a page of another origin it allows', async () => {
    const calculation = await session.calculateInPage(session.origin);

    expect(calculation).toEqual({
      sum: 42,
      quotient: { value: 3, remainder: 1 },
      failure: 'ByZero',
      count: { items: 100, end: 5050 },
      termSum: 5050,
      note: 'from a page'
    });
  });

  it('should be out of reach of a page of another origin it does not allow', async () => {
    const calculation = await session.calculateInPage();

    expect(calculation).toEqual({ closeError: 'ConnectionLoss' });
  });
});
