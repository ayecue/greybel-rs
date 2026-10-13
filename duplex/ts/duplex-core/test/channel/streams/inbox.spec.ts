import { describe, expect, it } from 'vitest';

import { FinalEventKind, Inbox } from '../../../src/channel/streams/inbox.js';

describe('Inbox', () => {
  it('should wake every reader waiting when the stream ends', async () => {
    const inbox = new Inbox();
    const waiting = Promise.all([inbox.wait(), inbox.wait()]);

    inbox.end({ kind: FinalEventKind.Value, value: 6n });
    await expect(waiting).resolves.toEqual([undefined, undefined]);
    expect(inbox.finalEvent).toEqual({ kind: FinalEventKind.Value, value: 6n });
  });

  it('should retain what ended it for after the items before it', () => {
    const inbox = new Inbox();

    inbox.push(1);
    inbox.end({ kind: FinalEventKind.Value, value: 1n });
    expect(inbox.finalEvent).toBeUndefined();
    expect(inbox.nextItem()).toBe(1);
    expect(inbox.finalEvent).toEqual({ kind: FinalEventKind.Value, value: 1n });
  });
});
