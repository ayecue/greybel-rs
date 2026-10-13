import { BatchStream } from '../../src/index.js';

export function streamed(...batches: number[][]): number[] {
  return batches.flatMap((batch) => [...BatchStream.getLengthPrefix(new Uint8Array(batch)), ...batch]);
}

export function batchesOf(stream: BatchStream, bytes: number[]): number[][] {
  const batches: number[][] = [];

  stream.process(new Uint8Array(bytes), (batch) => batches.push([...batch]));

  return batches;
}
