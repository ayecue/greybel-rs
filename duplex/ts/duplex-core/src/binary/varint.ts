import { CONTINUATION, LAST_U32_PAYLOAD, MAX_U32_BYTES, PAYLOAD, SHIFT } from '../const.js';

export const Varuint = {
  frameEnd(bytes: Uint8Array, offset: number): number | undefined {
    const payload = Varuint.getEnd(bytes, offset + 1);

    if (payload === undefined) return undefined;

    let length = 0;

    for (let index = offset + 1, shift = 0; index < payload; index++, shift += SHIFT) {
      length += (bytes[index]! & PAYLOAD) * 2 ** shift;
    }

    return payload + length;
  },

  getEnd(bytes: Uint8Array, offset: number): number | undefined {
    for (let index = 0; index < MAX_U32_BYTES; index++) {
      const byte = bytes[offset + index];

      if (byte === undefined) return undefined;
      if ((byte & CONTINUATION) !== 0) continue;

      return index === MAX_U32_BYTES - 1 && byte > LAST_U32_PAYLOAD ? undefined : offset + index + 1;
    }

    return undefined;
  }
};
