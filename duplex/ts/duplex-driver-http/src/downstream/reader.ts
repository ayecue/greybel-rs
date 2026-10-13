import { RECEIVED_HEADER, TRANSPORT_HEADER } from '../const.js';
import type { BatchListener } from '../records.js';

export interface DownstreamRequest {
  readonly url: string;
  readonly init: RequestInit;
}

export abstract class DownstreamReader {
  abstract readonly transport: string;
  protected readonly path: string = '';

  buildRequest(url: string, transportId: string, lastBatch: number): DownstreamRequest {
    return {
      url: url + this.path,
      init: { headers: { [TRANSPORT_HEADER]: transportId, [RECEIVED_HEADER]: String(lastBatch) } }
    };
  }

  abstract read(response: Response, onBatch: BatchListener): Promise<void>;
}
