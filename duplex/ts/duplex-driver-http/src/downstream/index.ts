import type { Downstream } from '../options.js';
import { ByteStream } from './byte-stream.js';
import type { DownstreamReader } from './reader.js';
import { EventStream } from './event-stream.js';
import { LongPoll } from './long-poll.js';

export { DownstreamReader } from './reader.js';

export const downstreamReaderFactories: Record<Downstream, () => DownstreamReader> = {
  'long-poll': () => new LongPoll(),
  events: () => new EventStream(),
  stream: () => new ByteStream()
};
