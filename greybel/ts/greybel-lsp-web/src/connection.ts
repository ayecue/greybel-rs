import { ErrorCode, type Next } from 'duplex-core';
import {
  AbstractMessageReader,
  AbstractMessageWriter,
  Disposable,
  type DataCallback,
  type Message,
  type MessageReader,
  type MessageWriter
} from 'vscode-jsonrpc/browser.js';

import { ClientMessage, ClientRouter, type ClientChannel, type ServerMessage } from './client.generated.js';

class ServerReader extends AbstractMessageReader implements MessageReader {
  #callback: DataCallback | undefined;

  listen(callback: DataCallback): Disposable {
    this.#callback = callback;

    return Disposable.create(() => {
      if (this.#callback !== callback) return;
      this.#callback = undefined;
    });
  }

  read(message: ServerMessage): void {
    try {
      this.#callback?.(JSON.parse(message.json) as Message);
    } catch (error) {
      this.fireError(error);
    }
  }

  close(): void {
    this.fireClose();
  }
}

class ServerRouter extends ClientRouter {
  readonly #reader: ServerReader;

  constructor(reader: ServerReader) {
    super();
    this.#reader = reader;
  }

  override onServerMessage(message: ServerMessage, _next: Next<void>): void {
    this.#reader.read(message);
  }
}

class ClientWriter extends AbstractMessageWriter implements MessageWriter {
  readonly #channel: ClientChannel;

  constructor(channel: ClientChannel) {
    super();
    this.#channel = channel;
  }

  async write(message: Message): Promise<void> {
    try {
      await this.#channel.send(new ClientMessage({ json: JSON.stringify(message) }));
    } catch (error) {
      this.fireError(error, message, 1);
      throw error;
    }
  }

  end(): void {
    this.#channel.close();
  }

  close(): void {
    this.fireClose();
  }
}

export class LanguageServerConnection {
  readonly channel: ClientChannel;
  readonly reader: ServerReader;
  readonly writer: ClientWriter;

  constructor(channel: ClientChannel) {
    this.reader = new ServerReader();
    this.channel = channel.use(new ServerRouter(this.reader));
    this.writer = new ClientWriter(this.channel);
    void this.channel.closeEvent.then(() => {
      this.reader.close();
      this.writer.close();
    });
  }

  close(): void {
    if (this.channel.closeReason !== undefined) return;
    this.channel.close(ErrorCode.ClosedChannel);
  }
}
