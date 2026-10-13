import { Capabilities, Driver, type Receiver } from 'duplex-core';

export interface WebSocketOptions {
  readonly url: string;

  readonly maxPendingBytes?: number;
  readonly WebSocket?: typeof WebSocket;
}

const MEBIBYTE = 1024 * 1024;
const NORMAL_CLOSURE = 1000;

export class WebSocketDriver extends Driver {
  readonly transport = 'websocket';
  readonly capabilities = Capabilities.CLIENT | Capabilities.PERSISTENT;
  readonly #url: string;
  readonly #maxPendingBytes: number;
  readonly #WebSocket: typeof WebSocket;
  #socket: WebSocket | undefined;
  #pendingBatches: Uint8Array[] = [];
  #receiver: Receiver | undefined;

  constructor({ url, maxPendingBytes, WebSocket: SocketConstructor }: WebSocketOptions) {
    super();
    this.#url = url;
    this.#maxPendingBytes = maxPendingBytes ?? 16 * MEBIBYTE;
    this.#WebSocket = SocketConstructor ?? globalThis.WebSocket;
  }

  open(receiver: Receiver): () => void {
    this.#stop();

    const socket = new this.#WebSocket(this.#url);

    socket.binaryType = 'arraybuffer';
    this.#socket = socket;
    this.#receiver = receiver;
    socket.onopen = () => {
      for (const batch of this.#pendingBatches.splice(0)) this.send(batch);
    };

    socket.onmessage = ({ data }: MessageEvent) => {
      if (!(data instanceof ArrayBuffer)) return;
      receiver.receive(new Uint8Array(data));
    };

    socket.onclose = () => this.#disconnect(socket);
    socket.onerror = () => this.#disconnect(socket);

    return () => {
      if (this.#socket !== socket) return;
      this.#stop();
    };
  }

  send(batch: Uint8Array): void {
    const socket = this.#socket;

    if (socket === undefined) return;
    if (socket.readyState === socket.CONNECTING) {
      this.#pendingBatches.push(batch);

      return;
    }

    if (socket.readyState !== socket.OPEN) return;
    if (socket.bufferedAmount + batch.byteLength > this.#maxPendingBytes) {
      this.#disconnect(socket);

      return;
    }

    socket.send(batch as Uint8Array<ArrayBuffer>);
  }

  #disconnect(socket: WebSocket): void {
    if (this.#socket !== socket) return;

    const receiver = this.#receiver;

    this.#stop();
    receiver?.onDisconnect();
  }

  #stop(): void {
    const socket = this.#socket;

    this.#socket = undefined;
    this.#receiver = undefined;
    this.#pendingBatches = [];
    if (socket === undefined) return;
    socket.onopen = socket.onmessage = socket.onclose = socket.onerror = null;
    if (socket.readyState !== socket.CONNECTING && socket.readyState !== socket.OPEN) return;
    socket.close(NORMAL_CLOSURE);
  }
}
