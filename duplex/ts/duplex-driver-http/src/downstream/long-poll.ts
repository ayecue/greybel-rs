import { NO_CONTENT } from '../const.js';
import { ServerError } from '../errors.js';
import { RecordReader, type BatchListener } from '../records.js';
import { DownstreamReader } from './reader.js';

export class LongPoll extends DownstreamReader {
  readonly transport = 'http';

  async read(response: Response, onBatch: BatchListener): Promise<void> {
    ServerError.ensureOk(response);
    if (response.status === NO_CONTENT) return;
    RecordReader.read(new Uint8Array(await response.arrayBuffer()), onBatch);
  }
}
