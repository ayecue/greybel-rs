import ansiEscapes from 'ansi-escapes';
import { Echo, formatEcho } from 'greybel-io-web';
import vscode, { Terminal, TerminalDimensions } from 'vscode';

const CLEAR = '\x1b[2J\x1b[3J\x1b[H';
const toCrlf = (text: string) => text.replace(/(?<!\r)\n/g, '\r\n');

export enum PseudoTerminalInputKey {
  Enter = '%0D',
  Backspace = '%7F',
  ArrowLeft = '%1B%5BD',
  ArrowRight = '%1B%5BC',
  ArrowUp = '%1B%5BA',
  ArrowDown = '%1B%5BB'
}

export default class PseudoTerminal {
  static activeTerminal: PseudoTerminal | null;

  public readonly name: string;
  public dimensions: TerminalDimensions;
  public terminal: Terminal;
  public outputEmitter: vscode.EventEmitter<string>;
  readonly #closeEvent = new vscode.EventEmitter<void>();
  readonly #input = new vscode.EventEmitter<string>();

  #active = true;
  private printedLineCount: number;

  static getActiveTerminal(): PseudoTerminal {
    return this.activeTerminal;
  }

  public constructor(name: string) {
    this.outputEmitter = new vscode.EventEmitter<string>();
    this.dimensions = null;
    this.terminal = null;
    this.name = name;
    this.printedLineCount = 0;

    this.createTerminal();
    PseudoTerminal.activeTerminal = this;
  }

  createTerminal() {
    this.terminal = vscode.window.createTerminal({
      name: this.name,
      pty: {
        onDidWrite: this.outputEmitter.event,
        open: (initialDimensions: TerminalDimensions) => {
          this.dimensions = initialDimensions;
        },
        setDimensions: (dimensions: TerminalDimensions) => {
          this.dimensions = dimensions;
        },
        close: () => {
          this.#closeEvent.fire();

          if (!this.#active) return;
          this.createTerminal();
        },
        handleInput: (input: string) => {
          this.#input.fire(input);
        }
      }
    });
  }

  waitForInput(endEvent: vscode.Event<void>, prompt: string, echo: Echo = Echo.Visible): PromiseLike<string> {
    this.focus();

    return new Promise((resolve) => {
      const buffer: string[] = [];
      let cursorIndex = 0;
      const shownIndex = () => (echo === Echo.Hidden ? 0 : cursorIndex);

      const cursorColumn = () => {
        const labelColumn = prompt.length % this.dimensions.columns;
        const firstLineWidth = this.dimensions.columns - labelColumn;
        const index = shownIndex();

        return index < firstLineWidth ? index : (index - this.dimensions.columns) % this.dimensions.columns;
      };

      const cursorRow = () => {
        const labelColumn = prompt.length % this.dimensions.columns;
        const firstLineWidth = this.dimensions.columns - labelColumn;
        const index = shownIndex();

        return index < firstLineWidth ? 0 : 1 + Math.floor((index - firstLineWidth) / this.dimensions.columns);
      };

      const removeListeners = () => {
        this.outputEmitter.fire('\r\n');
        listeners.dispose();
      };

      const onExit = () => {
        removeListeners();
        resolve('');
      };

      const onClose = () => {
        removeListeners();
        resolve('');
      };

      const onInput = (input: string) => {
        const code = encodeURI(input);

        switch (code) {
          case PseudoTerminalInputKey.Enter: {
            removeListeners();
            resolve(buffer.join(''));
            break;
          }

          case PseudoTerminalInputKey.Backspace:
            if (buffer.length > 0) {
              buffer.splice(cursorIndex - 1, 1);
              this.outputEmitter.fire(ansiEscapes.cursorRestorePosition);

              cursorIndex--;
              this.outputEmitter.fire(formatEcho(buffer.join(''), echo) + ' ');
              this.outputEmitter.fire(ansiEscapes.cursorRestorePosition);
              this.outputEmitter.fire(ansiEscapes.cursorMove(cursorColumn(), cursorRow()));
            }

            break;
          case PseudoTerminalInputKey.ArrowLeft:
            if (cursorIndex > 0) {
              cursorIndex--;
              this.outputEmitter.fire(ansiEscapes.cursorRestorePosition);
              this.outputEmitter.fire(ansiEscapes.cursorMove(cursorColumn(), cursorRow()));
            }

            break;
          case PseudoTerminalInputKey.ArrowRight:
            if (cursorIndex < buffer.length) {
              cursorIndex++;
              this.outputEmitter.fire(ansiEscapes.cursorRestorePosition);
              this.outputEmitter.fire(ansiEscapes.cursorMove(cursorColumn(), cursorRow()));
            }

            break;
          case PseudoTerminalInputKey.ArrowDown:
          case PseudoTerminalInputKey.ArrowUp:
            break;
          default: {
            buffer.splice(cursorIndex, 0, ...input);
            this.outputEmitter.fire(ansiEscapes.cursorRestorePosition);

            cursorIndex += input.length;
            this.outputEmitter.fire(formatEcho(buffer.join(''), echo));
            this.outputEmitter.fire(ansiEscapes.cursorRestorePosition);
            this.outputEmitter.fire(ansiEscapes.cursorMove(cursorColumn(), cursorRow()));
          }
        }
      };

      const listeners = vscode.Disposable.from(
        endEvent(onExit),
        this.#closeEvent.event(onClose),
        this.#input.event(onInput)
      );

      this.outputEmitter.fire(ansiEscapes.cursorSavePosition);
    });
  }

  waitForKeyPress(endEvent: vscode.Event<void>): PromiseLike<string> {
    this.focus();

    return new Promise((resolve) => {
      const removeListeners = () => listeners.dispose();

      const onExit = () => {
        removeListeners();
        resolve('');
      };

      const onInput = (input: string) => {
        removeListeners();
        resolve(input);
      };

      const onClose = () => {
        removeListeners();
        resolve('');
      };

      const listeners = vscode.Disposable.from(
        endEvent(onExit),
        this.#input.event(onInput),
        this.#closeEvent.event(onClose)
      );
    });
  }

  focus() {
    this.terminal.show(false);
  }

  hide() {
    this.terminal.hide();
  }

  print(message: string, appendNewLine: boolean = true) {
    const normalizedMessage = toCrlf(message);

    this.printedLineCount += normalizedMessage.split('\n').length;
    if (appendNewLine) this.printedLineCount++;

    this.outputEmitter.fire(`${normalizedMessage}${appendNewLine ? '\r\n' : ''}`);
    this.terminal.show();
  }

  replaceLastLine(message: string) {
    this.outputEmitter.fire(ansiEscapes.eraseLines(2));
    this.printedLineCount -= 2;
    this.print(message);
  }

  replacePrintedText(message: string) {
    this.outputEmitter.fire(ansiEscapes.eraseLines(this.printedLineCount));
    this.printedLineCount = 0;
    this.print(message);
  }

  clear() {
    this.outputEmitter.fire(CLEAR);
  }

  dispose() {
    if (!this.#active) return;

    this.#active = false;
    PseudoTerminal.activeTerminal = null;
    this.terminal.dispose();
  }
}
