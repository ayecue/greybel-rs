import { BatchStream, Capabilities, Driver, type Receiver } from 'duplex-core';

import { BATCHES_CONTENT_TYPE, NO_CONTENT, RECEIVED_HEADER, TRANSPORT_HEADER } from './const.js';
import { downstreamReaderFactories, type DownstreamReader } from './downstream/index.js';
import { ServerError } from './errors.js';
import { HTTP_STREAM_WINDOW, type HttpOptions } from './options.js';
import { RecordReader } from './records.js';

export class HttpDriver extends Driver {
  readonly transport: string;
  readonly capabilities = Capabilities.CLIENT | Capabilities.PERSISTENT;
  readonly streamWindow = HTTP_STREAM_WINDOW;
  readonly #url: string;
  readonly #downstream: DownstreamReader;
  readonly #fetch: typeof fetch;
  #transportId: string | undefined;
  #queue: Uint8Array[] = [];
  #postScheduled = false;
  readonly #startPost = (): void => {
    this.#postScheduled = false;
    void this.#post();
  };

  #posting = false;
  #lastBatch = 0;
  #receiver: Receiver | undefined;
  #controller: AbortController | undefined;

  constructor({ url, downstream, fetch: fetchFunction }: HttpOptions) {
    super();
    this.#downstream = downstreamReaderFactories[downstream]();
    this.transport = this.#downstream.transport;
    this.#url = url;
    this.#fetch = fetchFunction ?? globalThis.fetch.bind(globalThis);
  }

  open(receiver: Receiver): () => void {
    this.#controller?.abort();

    const controller = new AbortController();

    this.#controller = controller;
    this.#receiver = receiver;
    this.#transportId = undefined;
    this.#lastBatch = 0;

    return () => this.#stop(controller);
  }

  send(batch: Uint8Array): void {
    this.#queue.push(BatchStream.getLengthPrefix(batch), batch);
    if (this.#postScheduled) return;
    this.#postScheduled = true;
    queueMicrotask(this.#startPost);
  }

  async #post(): Promise<void> {
    const controller = this.#controller;

    if (this.#posting || controller === undefined) return;
    this.#posting = true;
    try {
      while (this.#queue.length > 0) {
        const headers: Record<string, string> = {
          [RECEIVED_HEADER]: String(this.#lastBatch),
          'content-type': BATCHES_CONTENT_TYPE
        };

        if (this.#transportId !== undefined) headers[TRANSPORT_HEADER] = this.#transportId;

        const init: RequestInit = { method: 'POST', headers, body: HttpDriver.#concat(this.#queue.splice(0)) };

        await this.#fetchWith(controller, this.#url, init, async (response) => {
          ServerError.ensureOk(response);
          if (this.#transportId === undefined)
            this.#onOpen(controller, ServerError.transportIdOf(response, TRANSPORT_HEADER));
          if (response.status === NO_CONTENT) return;
          RecordReader.read(new Uint8Array(await response.arrayBuffer()), this.#onBatch);
        });
      }
    } catch {
      this.#disconnect(controller);
    } finally {
      this.#posting = false;
    }
  }

  #onOpen(controller: AbortController, transportId: string): void {
    if (this.#controller !== controller) return;
    this.#transportId = transportId;
    void this.#readDownstream(controller, transportId);
  }

  async #readDownstream(controller: AbortController, transportId: string): Promise<void> {
    try {
      while (!controller.signal.aborted) {
        const { url, init } = this.#downstream.buildRequest(this.#url, transportId, this.#lastBatch);

        await this.#fetchWith(controller, url, init, (response) => this.#downstream.read(response, this.#onBatch));
      }
    } catch {
      this.#disconnect(controller);
    }
  }

  async #fetchWith(
    controller: AbortController,
    url: string,
    init: RequestInit,
    handleResponse: (response: Response) => Promise<void>
  ): Promise<void> {
    if (controller.signal.aborted) throw controller.signal.reason;

    const requestController = new AbortController();
    const abortRequest = () => requestController.abort(controller.signal.reason);

    controller.signal.addEventListener('abort', abortRequest, { once: true });
    try {
      await handleResponse(await this.#fetch(url, { ...init, signal: requestController.signal }));
    } finally {
      controller.signal.removeEventListener('abort', abortRequest);
    }
  }

  readonly #onBatch = (sequence: number, batch: Uint8Array): void => {
    if (sequence !== this.#lastBatch + 1) return;
    this.#lastBatch = sequence;
    this.#receiver?.receive(batch);
  };

  #disconnect(controller: AbortController): void {
    if (controller.signal.aborted) return;

    const receiver = this.#receiver;

    this.#stop(controller);
    receiver?.onDisconnect();
  }

  #stop(controller: AbortController): void {
    controller.abort();
    if (this.#controller !== controller) return;
    this.#controller = undefined;
    this.#receiver = undefined;
    this.#queue = [];
  }

  static #concat(parts: Uint8Array[]): Uint8Array<ArrayBuffer> {
    const concatenation = new Uint8Array(parts.reduce((length, part) => length + part.byteLength, 0));
    let offset = 0;

    for (const part of parts) {
      concatenation.set(part, offset);
      offset += part.byteLength;
    }

    return concatenation;
  }
}
