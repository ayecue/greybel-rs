import { describe, expect, it } from 'vitest';

import { ReceiveWindow } from '../../../src/channel/streams/receive-window.js';
import { INITIAL_CREDIT } from '../../../src/const.js';
import { grants } from '../../support/receive-window.js';

describe('ReceiveWindow', () => {
  it('should grant its window past the initial credit as its stream starts', () => {
    const granted = new ReceiveWindow(32, 8).initialGrant();

    expect(granted).toBe(32 - INITIAL_CREDIT);
  });

  it('should grant what it took once it took half its window', () => {
    const granted = grants(new ReceiveWindow(32, 8), 64, 8);

    expect(granted).toEqual([31, 16, 16, 16, 16]);
  });

  it('should fit its window to the budget at the declared item size', () => {
    const granted = grants(new ReceiveWindow(4096, 64 * 1024), 16, 64 * 1024);

    expect(granted).toEqual([15, 8, 8]);
  });

  it('should shrink its window to the largest item it received', () => {
    const granted = grants(new ReceiveWindow(4096, undefined), 256, 256 * 1024);

    expect(granted).toEqual([255, 2, 2]);
  });

  it('should grow its window for small items up to its most', () => {
    const granted = grants(new ReceiveWindow(4096, undefined), 1, 4);

    expect(granted).toEqual([255, 4096 - 255]);
  });

  it('should grant once every item was taken with a window of one', () => {
    const granted = grants(new ReceiveWindow(4096, 2 * 1024 * 1024), 3, 2 * 1024 * 1024);

    expect(granted).toEqual([1, 1, 1]);
  });
});
