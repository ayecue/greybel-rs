import { BatchStream } from 'duplex-core';

export function stubReceiver(): {
  batches: number[][];
  disconnections: number;
  receive(batch: Uint8Array): void;
  onDisconnect(): void;
} {
  const batches: number[][] = [];
  const receiver = {
    batches,
    disconnections: 0,
    receive: (batch: Uint8Array) => batches.push([...batch]),
    onDisconnect: () => {
      receiver.disconnections += 1;
    }
  };

  return receiver;
}

export function encode(batch: number[]): Uint8Array {
  return new Uint8Array([...BatchStream.getLengthPrefix(new Uint8Array(batch)), ...batch]);
}
