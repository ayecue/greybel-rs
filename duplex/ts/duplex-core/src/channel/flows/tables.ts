import { DENSE_ID_LIMIT } from '../../const.js';
import type { IncomingFlow } from './inbound.js';
import type { OutgoingFlow } from './outbound.js';

export class OutgoingFlowTable {
  readonly #flows: (OutgoingFlow | undefined)[] = [];
  readonly #vacantIds: number[] = [];
  #size = 0;

  get size(): number {
    return this.#size;
  }

  get nextId(): number {
    return this.#vacantIds.at(-1) ?? this.#flows.length;
  }

  get(exchangeId: number): OutgoingFlow | undefined {
    return this.#flows[exchangeId];
  }

  add(flow: OutgoingFlow): void {
    if (flow.exchangeId === this.#vacantIds.at(-1)) this.#vacantIds.pop();
    this.#flows[flow.exchangeId] = flow;
    this.#size++;
  }

  remove(flow: OutgoingFlow): void {
    if (this.#flows[flow.exchangeId] !== flow) return;
    this.#flows[flow.exchangeId] = undefined;
    this.#vacantIds.push(flow.exchangeId);
    this.#size--;
  }

  toArray(): OutgoingFlow[] {
    return this.#flows.filter((flow) => flow !== undefined);
  }
}

export class IncomingFlowTable {
  #dense: (IncomingFlow | undefined)[] = [];
  #sparse: Map<number, IncomingFlow> | undefined;
  #size = 0;
  #readyCount = 0;

  get size(): number {
    return this.#size;
  }

  get readyCount(): number {
    return this.#readyCount;
  }

  get busyCount(): number {
    return this.#size - this.#readyCount;
  }

  onBusy(flow: IncomingFlow): void {
    if (this.get(flow.exchangeId) !== flow) return;
    this.#readyCount--;
  }

  get(exchangeId: number): IncomingFlow | undefined {
    return exchangeId < DENSE_ID_LIMIT ? this.#dense[exchangeId] : this.#sparse?.get(exchangeId);
  }

  add(flow: IncomingFlow): void {
    if (flow.exchangeId < DENSE_ID_LIMIT) this.#dense[flow.exchangeId] = flow;
    else (this.#sparse ??= new Map()).set(flow.exchangeId, flow);
    this.#size++;
    this.#readyCount++;
  }

  remove(flow: IncomingFlow): void {
    if (this.get(flow.exchangeId) !== flow) return;
    if (flow.exchangeId < DENSE_ID_LIMIT) this.#dense[flow.exchangeId] = undefined;
    else this.#sparse!.delete(flow.exchangeId);
    this.#size--;
  }

  toArray(): IncomingFlow[] {
    const flows = this.#dense.filter((flow) => flow !== undefined);

    if (this.#sparse !== undefined) flows.push(...this.#sparse.values());

    return flows;
  }

  drain(): IncomingFlow[] {
    const flows = this.#dense.filter((flow) => flow !== undefined);

    if (this.#sparse !== undefined) flows.push(...this.#sparse.values());
    this.#dense = [];
    this.#sparse = undefined;
    this.#size = 0;
    this.#readyCount = 0;

    return flows;
  }
}
