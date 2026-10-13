import { ChannelError } from 'duplex-core';
import { Key, Output, PageRouter, ReadFailure, type OutputBatch, type ReadKey, type ReadLine } from 'greybel-io-web';
import { Render, RichText, type TerminalChannel } from 'greyhack-terminal-text';

import PseudoTerminal from '../debug/pseudo-terminal';
import { getPreview } from '../preview';
import { toKey } from './terminal-keys';
import { EventEmitter } from 'vscode';

export class TerminalPage extends PageRouter {
  readonly terminal: PseudoTerminal;
  readonly #channel: TerminalChannel;
  readonly #endEvent = new EventEmitter<void>();

  #printQueue: Promise<void> = Promise.resolve();
  #isOver = false;

  constructor(channel: TerminalChannel) {
    super();
    PseudoTerminal.activeTerminal?.dispose();
    this.terminal = new PseudoTerminal('greybel');
    this.#channel = channel;
    this.terminal.focus();
  }

  override onOutputBatch(message: OutputBatch): void {
    getPreview().show(message.entries);
    this.#show(message.entries);
  }

  override async onReadLine(message: ReadLine): Promise<string> {
    await this.#showPrompt(message.prompt);

    const line = await this.terminal.waitForInput(this.#endEvent.event, message.prompt, message.echo);

    if (this.#isOver) throw ChannelError.fail(ReadFailure.Cancellation);

    return line;
  }

  override async onReadKey(message: ReadKey): Promise<Key> {
    await this.#showPrompt(message.prompt);

    const input = await this.terminal.waitForKeyPress(this.#endEvent.event);

    if (this.#isOver || input === '') throw ChannelError.fail(ReadFailure.Cancellation);

    return toKey(input);
  }

  printError(text: string): void {
    this.#printQueue = this.#printQueue.then(() => this.terminal.print(`\x1b[31m${text}\x1b[0m`));
  }

  end(): void {
    this.#isOver = true;
    this.#endEvent.fire();
  }

  #showPrompt(prompt: string): Promise<void> {
    const entry = new Output.Print({ text: prompt, newLine: false, replace: false });

    getPreview().show([entry]);

    return this.#show([entry]);
  }

  #show(entries: Output[]): Promise<void> {
    const prints = entries.filter((entry) => entry instanceof Output.Print);

    const columns = this.terminal.dimensions?.columns ?? 0;
    const ansiTexts = this.#render(
      prints.map((print) => new RichText({ text: print.text, columns: print.newLine ? columns : 0 }))
    );

    this.#printQueue = this.#printQueue.then(async () => {
      const texts = await ansiTexts;
      let index = 0;

      for (const entry of entries) {
        if (entry instanceof Output.Clear) {
          this.terminal.clear();
        } else if (entry instanceof Output.Print) {
          const text = texts[index++]!;

          if (entry.replace) this.terminal.replaceLastLine(text);
          else this.terminal.print(text, entry.newLine);
        }
      }
    });

    return this.#printQueue;
  }

  async #render(texts: RichText[]): Promise<string[]> {
    if (texts.length === 0) return [];
    try {
      return (await this.#channel.send(new Render({ texts }))).texts;
    } catch {
      return texts.map(({ text }) => text);
    }
  }
}
