import type { Next } from 'duplex-core';

import { PageRouter } from '../page.generated.js';
import { Output, type Key, type OutputBatch, type ReadKey, type ReadLine } from '../protocol.generated.js';
import { formatEcho } from './echo.js';
import { TextScreen } from './text.js';

export class ScreenOutput extends PageRouter {
  readonly screen: TextScreen;
  readonly #render: (screen: TextScreen) => void;

  constructor(render: (screen: TextScreen) => void, screen = new TextScreen()) {
    super();
    this.#render = render;
    this.screen = screen;
  }

  override onOutputBatch(message: OutputBatch): void {
    this.screen.apply(message.entries);
    this.#render(this.screen);
  }

  override async onReadLine(request: ReadLine, next: Next<string>): Promise<string> {
    this.#showPrompt(request.prompt);

    const line = await next();

    this.#print(new Output.Print({ text: formatEcho(line, request.echo), newLine: true, replace: false }));

    return line;
  }

  override onReadKey(request: ReadKey, next: Next<Key>): Promise<Key> {
    this.#showPrompt(request.prompt);

    return next();
  }

  #showPrompt(prompt: string): void {
    if (prompt === '') return;
    this.#print(new Output.Print({ text: prompt, newLine: false, replace: false }));
  }

  #print(output: Output.Print): void {
    this.screen.print(output);
    this.#render(this.screen);
  }
}
