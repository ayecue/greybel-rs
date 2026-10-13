import { describe, expect, it } from 'vitest';

import {
  ChannelSettings,
  DEFAULT_HEALTH_CHECK,
  DEFAULT_MAX_MESSAGE,
  DEFAULT_STREAM_WINDOW
} from '../../src/channel/options.js';
import { Login } from '../login.generated.js';

describe('flavours', () => {
  it('should take the window of the options, then of the flavour, then of the driver', () => {
    const windows = [
      new ChannelSettings({ streamWindow: 8 }, { streamWindow: 64 }, 1024),
      new ChannelSettings({}, { streamWindow: 64 }, 1024),
      new ChannelSettings({}, undefined, 1024)
    ].map((settings) => settings.streamWindow);

    expect(windows).toEqual([8, 64, 1024]);
  });

  it('should take the timeout of the flavour', () => {
    const timeouts = [new ChannelSettings({}, { timeout: 2000 }, 1024), new ChannelSettings({}, undefined, 1024)].map(
      (settings) => settings.timeout
    );

    expect(timeouts).toEqual([2000, undefined]);
  });

  it('should take the most a message takes of the options, then of the flavour, then the default', () => {
    const most = [
      new ChannelSettings({ maxMessage: 16 }, { maxMessage: 1024 }, 1024),
      new ChannelSettings({}, { maxMessage: 1024 }, 1024),
      new ChannelSettings({}, undefined, 1024)
    ].map((settings) => settings.maxMessage);

    expect(most).toEqual([16, 1024, DEFAULT_MAX_MESSAGE]);
  });

  it('should check health as the flavour of the transport says, after the options', () => {
    const { flavours } = Login;
    const checks = [
      new ChannelSettings({}, flavours.stdio, DEFAULT_STREAM_WINDOW),
      new ChannelSettings({}, flavours.port, DEFAULT_STREAM_WINDOW),
      new ChannelSettings({}, undefined, DEFAULT_STREAM_WINDOW),
      new ChannelSettings({ healthCheck: false }, flavours.stdio, DEFAULT_STREAM_WINDOW),
      new ChannelSettings(
        { healthCheck: { interval: 9, unhealthyThreshold: 1, closeThreshold: 2 } },
        flavours.port,
        DEFAULT_STREAM_WINDOW
      )
    ].map((settings) => settings.healthCheck);

    expect(checks).toEqual([
      { ...DEFAULT_HEALTH_CHECK, interval: 1000, closeThreshold: 7 },
      undefined,
      DEFAULT_HEALTH_CHECK,
      undefined,
      { interval: 9, unhealthyThreshold: 1, closeThreshold: 2 }
    ]);
  });
});
