import {
  DATA_FIELD,
  EVENT_END,
  EVENTS_PATH,
  FIELD_SEPARATOR,
  ID_FIELD,
  LINE_END,
  NATIVE_BASE64_MIN_LENGTH
} from '../const.js';
import { ServerError } from '../errors.js';
import type { BatchListener } from '../records.js';
import { DownstreamReader } from './reader.js';

type FromBase64 = (text: string) => Uint8Array;
type NodeBuffer = { from(text: string, encoding: 'base64'): Uint8Array };

const fromBase64 = (Uint8Array as unknown as { fromBase64?: FromBase64 }).fromBase64;
const NodeBuffer = (globalThis as unknown as { Buffer?: NodeBuffer }).Buffer;

export class EventStream extends DownstreamReader {
  readonly transport = 'sse';

  protected override readonly path = EVENTS_PATH;

  async read(response: Response, onBatch: BatchListener): Promise<void> {
    const reader = ServerError.getBody(response).getReader();
    const parser = new EventStreamParser(onBatch);

    for (;;) {
      const { done, value } = await reader.read();

      if (done) return;
      parser.process(value);
    }
  }
}

class EventStreamParser {
  readonly #decoder = new TextDecoder();
  readonly #onBatch: BatchListener;
  #pendingText = '';

  constructor(onBatch: BatchListener) {
    this.#onBatch = onBatch;
  }

  process(bytes: Uint8Array): void {
    const text = (this.#pendingText += this.#decoder.decode(bytes, { stream: true }));
    let start = 0;

    for (let end = text.indexOf(EVENT_END); end >= 0; end = text.indexOf(EVENT_END, start)) {
      this.#parseEvent(text, start, end);
      start = end + EVENT_END.length;
    }

    if (!(start > 0)) return;
    this.#pendingText = text.slice(start);
  }

  #parseEvent(text: string, start: number, end: number): void {
    let id: number | undefined;
    let data = '';

    for (let line = start; line < end;) {
      const next = text.indexOf(LINE_END, line);
      const lineEnd = next < 0 || next > end ? end : next;
      const separator = text.indexOf(FIELD_SEPARATOR, line);

      if (separator > line && separator < lineEnd) {
        const field = text.slice(line, separator);
        const value = text.slice(separator + FIELD_SEPARATOR.length, lineEnd);

        if (field === ID_FIELD) id = Number(value);
        else if (field === DATA_FIELD) data += value;
      }

      line = lineEnd + LINE_END.length;
    }

    if (id === undefined) return;
    this.#onBatch(id, EventStreamParser.#decodeBase64(data));
  }

  static #decodeBase64(base64: string): Uint8Array {
    if (fromBase64 !== undefined && base64.length >= NATIVE_BASE64_MIN_LENGTH) return fromBase64(base64);
    if (NodeBuffer !== undefined) return NodeBuffer.from(base64, 'base64');

    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);

    for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);

    return bytes;
  }
}
