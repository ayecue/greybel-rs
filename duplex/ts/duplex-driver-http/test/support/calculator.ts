import { HttpDriver, type Downstream } from '../../src/index.js';
import { Progress, UserChannel, UserRouter } from '../calculator.client.generated.js';
import { CalculatorExample } from './example.js';

export class NoteCollector extends UserRouter {
  readonly notes: string[] = [];
  #resolveNote: (() => void) | undefined;

  override onProgress(message: Progress): void {
    if (typeof message.note === 'string') this.notes.push(message.note);
    this.#resolveNote?.();
  }

  nextNote(): Promise<void> {
    return new Promise((resolve) => {
      this.#resolveNote = resolve;
    });
  }
}

export const calculators: { calculator: CalculatorExample; user?: UserChannel }[] = [];

export async function calculate(
  downstream: Downstream,
  fetching?: typeof fetch
): Promise<{ user: UserChannel; notes: NoteCollector }> {
  const calculator = await CalculatorExample.start(downstream);
  const notes = new NoteCollector();
  const driver = new HttpDriver({ url: calculator.url, downstream, fetch: fetching });
  const user = new UserChannel(driver, { healthCheck: false }).use(notes);

  calculators.push({ calculator, user });

  return { user, notes };
}
