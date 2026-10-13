import { createServer, type Server } from 'node:http';
import type { AddressInfo } from 'node:net';
import { fileURLToPath } from 'node:url';

import { build } from 'esbuild';
import { chromium, type Browser } from 'playwright';

import { CalculatorExample } from './example.js';
import type { CalculationReport } from './page.js';

const PAGE = fileURLToPath(new URL('./page.ts', import.meta.url));

class PageServer {
  private constructor(
    private readonly server: Server,
    readonly url: string
  ) {}

  static async start(script: string): Promise<PageServer> {
    const server = createServer((request, response) => {
      if (request.url === '/page.js') {
        response.writeHead(200, { 'content-type': 'text/javascript' });
        response.end(script);

        return;
      }

      response.writeHead(200, { 'content-type': 'text/html' });
      response.end('<!doctype html><script src="/page.js"></script>');
    });

    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));

    const { port } = server.address() as AddressInfo;

    return new PageServer(server, `http://127.0.0.1:${port}/`);
  }

  get origin(): string {
    return new URL(this.url).origin;
  }

  close(): Promise<void> {
    return new Promise((resolve) => this.server.close(() => resolve()));
  }
}

export class BrowserSession {
  #pageServer: PageServer | undefined;
  #browser: Browser | undefined;
  readonly #calculators: CalculatorExample[] = [];

  async start(): Promise<void> {
    CalculatorExample.build();

    const bundled = await build({
      entryPoints: [PAGE],
      bundle: true,
      format: 'iife',
      globalName: 'calculatorPage',
      platform: 'browser',
      target: 'es2022',
      conditions: ['source'],
      write: false,
      logLevel: 'warning'
    });

    this.#pageServer = await PageServer.start(bundled.outputFiles[0]!.text);
    this.#browser = await chromium.launch();
  }

  get origin(): string {
    return this.#startedServer().origin;
  }

  async calculateInPage(allowOrigin?: string): Promise<CalculationReport> {
    const calculator = await CalculatorExample.start(allowOrigin);

    this.#calculators.push(calculator);

    const page = await this.#startedBrowser().newPage();

    try {
      await page.goto(this.#startedServer().url);

      return await page.evaluate(({ url }) => calculatorPage.calculate(url), { url: calculator.url });
    } finally {
      await page.close();
    }
  }

  stopCalculators(): void {
    for (const calculator of this.#calculators.splice(0)) calculator.stop();
  }

  async close(): Promise<void> {
    await this.#browser?.close();
    await this.#pageServer?.close();
  }

  #startedServer(): PageServer {
    if (this.#pageServer === undefined) throw new Error('the session has not started');

    return this.#pageServer;
  }

  #startedBrowser(): Browser {
    if (this.#browser === undefined) throw new Error('the session has not started');

    return this.#browser;
  }
}

export const session = new BrowserSession();
