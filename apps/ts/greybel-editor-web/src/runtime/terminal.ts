import { ChannelError } from 'duplex-core';
import { MessageTargetDriver } from 'duplex-driver-worker';
import {
  formatEcho,
  Key,
  KeyMap,
  Output,
  PageRouter,
  ReadFailure,
  TextScreen,
  type OutputBatch as PageOutputBatch,
  type ReadKey,
  type ReadLine
} from 'greybel-io-web';
import { OutputBatch, ProgramChannel } from 'greybel-io-web/program';
import { openTerminal, type TerminalPage } from 'greyhack-terminal-web';
import terminalWasm from 'greyhack-terminal-web/greyhack-terminal-web.wasm?url';

import type { WasmLoader } from '../wasm.js';

interface PendingRead {
  readonly key: (key: Key) => void;
  readonly cancel: () => void;
}

/** Implements the program's io page; greyhack-terminal-web's worker draws the output. */
export class Terminal extends PageRouter {
  readonly #canvas: HTMLCanvasElement;
  /** Opened once the terminal's module is compiled; see `startWorker`. */
  readonly #screen: Promise<{ readonly page: TerminalPage; readonly channel: ProgramChannel }>;
  readonly #text = new TextScreen();
  readonly #onText: (text: string) => void;

  #read: PendingRead | undefined;

  readonly #onKeyDown = (event: KeyboardEvent) => {
    if (this.#read === undefined) return;

    const key = KeyMap.of(event);

    if (key === undefined) return;
    event.preventDefault();
    this.#read.key(key);
  };

  constructor(canvas: HTMLCanvasElement, onText: (text: string) => void, wasm: WasmLoader) {
    super();

    const { port1, port2 } = new MessageChannel();

    this.#canvas = canvas;
    this.#onText = onText;
    this.#screen = wasm.load('terminal', terminalWasm).then((module) => ({
      page: openTerminal(canvas, port1, module),
      channel: new ProgramChannel(new MessageTargetDriver(port2))
    }));
    canvas.tabIndex = 0;
    canvas.addEventListener('keydown', this.#onKeyDown);
  }

  get isReading(): boolean {
    return this.#read !== undefined;
  }

  override onOutputBatch(message: PageOutputBatch): void {
    this.#show(message.entries);
  }

  override onReadLine(message: ReadLine): Promise<string> {
    let line = '';
    const showLine = (replace: boolean) =>
      this.#print(`${message.prompt}${formatEcho(line, message.echo)}`, {
        newLine: false,
        replace
      });

    showLine(false);

    return this.#readUntil((key, resolve) => {
      if (key instanceof Key.Enter) {
        this.#print('', { newLine: true, replace: false });
        resolve(line);

        return;
      }

      if (key instanceof Key.Char) line += key.value;
      else if (key instanceof Key.Backspace) line = [...line].slice(0, -1).join('');
      else return;

      showLine(true);
    });
  }

  override onReadKey(message: ReadKey): Promise<Key> {
    this.#print(message.prompt, { newLine: false, replace: false });

    return this.#readUntil((key, resolve) => resolve(key));
  }

  printLine(text: string): void {
    this.#print(text, { newLine: true, replace: false });
  }

  clear(): void {
    this.#show([new Output.Clear()]);
  }

  cancelRead(): void {
    this.#read?.cancel();
  }

  close(): void {
    this.cancelRead();
    this.#canvas.removeEventListener('keydown', this.#onKeyDown);
    void this.#screen.then(({ page, channel }) => {
      channel.close();
      page.close();
    });
  }

  #readUntil<T>(onKey: (key: Key, resolve: (value: T) => void) => void): Promise<T> {
    this.#canvas.focus();

    return new Promise<T>((resolve, reject) => {
      const end = () => {
        this.#read = undefined;
      };

      this.#read = {
        key: (key) =>
          onKey(key, (value) => {
            end();
            resolve(value);
          }),
        cancel: () => {
          end();
          reject(ChannelError.fail(ReadFailure.Cancellation));
        }
      };
    });
  }

  #print(text: string, options: { readonly newLine: boolean; readonly replace: boolean }): void {
    this.#show([new Output.Print({ text, ...options })]);
  }

  #show(entries: Output[]): void {
    void this.#screen.then(({ channel }) => channel.send(new OutputBatch({ entries })));
    this.#text.apply(entries);
    this.#onText(this.#text.text);
  }
}
