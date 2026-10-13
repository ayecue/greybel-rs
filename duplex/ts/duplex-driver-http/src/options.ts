export type Downstream = 'long-poll' | 'events' | 'stream';

export interface HttpOptions {
  readonly url: string;
  readonly downstream: Downstream;
  readonly fetch?: typeof fetch;
}

export const HTTP_STREAM_WINDOW = 16 * 1024;
