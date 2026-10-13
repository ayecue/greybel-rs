import { BatchStream } from 'duplex-core';

import { VARUINT_CONTINUATION, VARUINT_PAYLOAD_MASK } from './const.js';

export type BatchListener = (sequence: number, batch: Uint8Array) => void;

export class RecordReader {
  readonly #batchStream = new BatchStream();
  readonly #onBatch: BatchListener;

  constructor(onBatch: BatchListener) {
    this.#onBatch = onBatch;
  }

  static read(body: Uint8Array, onBatch: BatchListener): void {
    new RecordReader(onBatch).process(body);
  }

  process(bytes: Uint8Array): void {
    this.#batchStream.process(bytes, (record) => this.#readRecord(record));
  }

  #readRecord(record: Uint8Array): void {
    if (record.byteLength === 0) return;

    let sequence = 0;
    let index = 0;

    for (let scale = 1; index < record.byteLength; scale *= VARUINT_CONTINUATION) {
      const byte = record[index++]!;

      sequence += (byte & VARUINT_PAYLOAD_MASK) * scale;
      if ((byte & VARUINT_CONTINUATION) === 0) break;
    }

    this.#onBatch(sequence, record.subarray(index));
  }
}
