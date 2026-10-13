import { STREAM_PATH } from '../const.js';
import { ServerError } from '../errors.js';
import { RecordReader, type BatchListener } from '../records.js';
import { DownstreamReader } from './reader.js';

export class ByteStream extends DownstreamReader {
  readonly transport = 'http_stream';

  protected override readonly path = STREAM_PATH;

  async read(response: Response, onBatch: BatchListener): Promise<void> {
    const reader = ServerError.getBody(response).getReader();
    const recordReader = new RecordReader(onBatch);

    for (;;) {
      const { done, value } = await reader.read();

      if (done) return;
      recordReader.process(value);
    }
  }
}
