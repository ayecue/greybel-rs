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

/** Pending user-input read, ended by a keypress or program exit. */
interface PendingRead {
  readonly key: (key: Key) => void;
  readonly cancel: () => void;
}

/**
 * Canvas-based program terminal matching Grey Hack output and keyboard input. The page implements the
 * program's io page and relays output to greyhack-terminal-web's worker. `onText` also receives text for
 * canvas-inaccessible consumers such as screen readers.
 */
export class Terminal extends PageRouter {
  readonly #canvas: HTMLCanvasElement;
  readonly #page: TerminalPage;
  readonly #screen: ProgramChannel;
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

  constructor(canvas: HTMLCanvasElement, onText: (text: string) => void) {
    super();

    const { port1, port2 } = new MessageChannel();

    this.#canvas = canvas;
    this.#onText = onText;
    this.#page = openTerminal(canvas, port1, terminalWasm);
    this.#screen = new ProgramChannel(new MessageTargetDriver(port2));
    canvas.tabIndex = 0;
    canvas.addEventListener('keydown', this.#onKeyDown);
  }

  /** Whether the program is waiting for user input. */
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

  /** Write `text` on its own line, for example a program failure reason. */
  printLine(text: string): void {
    this.#print(text, { newLine: true, replace: false });
  }

  clear(): void {
    this.#show([new Output.Clear()]);
  }

  /** Cancel the active read because the program ended. */
  cancelRead(): void {
    this.#read?.cancel();
  }

  close(): void {
    this.cancelRead();
    this.#canvas.removeEventListener('keydown', this.#onKeyDown);
    this.#screen.close();
    this.#page.close();
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
    void this.#screen.send(new OutputBatch({ entries }));
    this.#text.apply(entries);
    this.#onText(this.#text.text);
  }
}
