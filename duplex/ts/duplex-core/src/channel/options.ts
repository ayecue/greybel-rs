import { COMMON_BUNDLE_ID } from '../const.js';
import { DEFAULT_STREAM_WINDOW } from '../driver/index.js';
import type { TransportFlavour } from '../protocol/index.js';

export { DEFAULT_STREAM_WINDOW };

export interface HealthCheck {
  interval: number;
  unhealthyThreshold: number;
  closeThreshold: number;
}

export interface ChannelOptions {
  healthCheck?: HealthCheck | false;
  bundle?: Bundle;
  limits?: Partial<Limits>;
  streamWindow?: number;

  maxMessage?: number;

  unvalidated?: boolean;
}

export interface Limits {
  readonly queuedItems: number;
  readonly pendingItems: number;
  readonly activeResponses: number;
  readonly readyItems: number;
}

export const DEFAULT_LIMITS: Limits = {
  queuedItems: 2048,
  pendingItems: 16_384,
  activeResponses: 8192,
  readyItems: 16_384
};

export type Bundle = 'common' | { readonly key: number };

export const Bundle = {
  COMMON: 'common' as Bundle,

  key(key: number): Bundle {
    return { key };
  },

  id(bundle: Bundle): number {
    return bundle === 'common' ? COMMON_BUNDLE_ID : bundle.key;
  }
};

export const DEFAULT_MAX_MESSAGE = 64 * 1024 * 1024;

export const DEFAULT_HEALTH_CHECK: HealthCheck = { interval: 5_000, unhealthyThreshold: 2, closeThreshold: 3 };

export class ChannelSettings {
  readonly healthCheck: HealthCheck | undefined;
  readonly streamWindow: number;
  readonly bundle: Bundle;
  readonly limits: Limits;
  readonly timeout: number | undefined;
  readonly maxMessage: number;
  readonly validating: boolean;

  constructor(options: ChannelOptions, flavour: TransportFlavour | undefined, window: number) {
    const healthCheck = options.healthCheck ?? flavour?.healthCheck;

    this.healthCheck = healthCheck === false ? undefined : { ...DEFAULT_HEALTH_CHECK, ...healthCheck };
    this.streamWindow = options.streamWindow ?? flavour?.streamWindow ?? window;
    this.bundle = options.bundle ?? Bundle.COMMON;
    this.limits = { ...DEFAULT_LIMITS, ...options.limits };
    this.timeout = flavour?.timeout;
    this.maxMessage = options.maxMessage ?? flavour?.maxMessage ?? DEFAULT_MAX_MESSAGE;
    this.validating = options.unvalidated !== true;
  }
}
