import { ChannelError, ErrorCode } from 'duplex-core';

import { HttpDriver, type Downstream } from '../../src/index.js';
import {
  Add,
  Count,
  Divide,
  DivisionFailure,
  Log,
  Progress,
  Quotient,
  Sum,
  UserChannel,
  UserRouter
} from '../calculator.client.generated.js';

export interface CalculationReport {
  readonly sum?: number;
  readonly quotient?: { value: number; remainder: number };
  readonly failure?: string;
  readonly count?: { items: number; end: number };
  readonly termSum?: number;
  readonly note?: string;
  readonly closeError?: string;
}

class NoteCollector extends UserRouter {
  readonly note: Promise<string>;
  #resolveNote!: (note: string) => void;

  constructor() {
    super();
    this.note = new Promise((resolve) => {
      this.#resolveNote = resolve;
    });
  }

  override onProgress(message: Progress): void {
    if (typeof message.note !== 'string') return;
    this.#resolveNote(message.note);
  }
}

export async function calculate(url: string, downstream: Downstream): Promise<CalculationReport> {
  const notes = new NoteCollector();
  const user = new UserChannel(new HttpDriver({ url, downstream }), { healthCheck: false }).use(notes);

  try {
    const sum = Number(await user.send(new Add({ a: 20n, b: 22n })));
    const quotient = (await user.send(new Divide({ dividend: 7n, divisor: 2n }))) as Quotient.WithRemainder;
    const failure = await user.send(new Divide({ dividend: 7n, divisor: 0n })).then(
      () => 'none',
      (error: unknown) =>
        error instanceof ChannelError ? DivisionFailure[error.failure as DivisionFailure] : String(error)
    );
    const stream = user.send(new Count({ to: 100 }));

    await stream.start();

    let items = 0;

    for await (const _ of stream) items++;

    const count = { items, end: Number(await stream.end()) };
    const { stream: terms, response } = user.send(new Sum());

    for (let term = 1n; term <= 100n; term++) await terms.send(term);
    await terms.end(undefined);

    const termSum = Number(await response);

    user.send(new Log({ text: 'from a page' }));

    const note = await notes.note;

    return {
      sum,
      quotient: { value: Number(quotient.value), remainder: Number(quotient.remainder) },
      failure,
      count,
      termSum,
      note
    };
  } catch (error) {
    if (error instanceof ChannelError && error.code !== undefined) return { closeError: ErrorCode[error.code] };
    throw error;
  } finally {
    user.close();
  }
}
