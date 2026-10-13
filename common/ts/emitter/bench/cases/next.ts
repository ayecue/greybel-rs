import { EventEmitter } from "node:events";
import { do_not_optimize } from "mitata";
import { NextEmitter } from "../../src/index.js";
import type { BenchGroup } from "../case.js";

const WAITERS = 16;

class Next extends NextEmitter {}

function listener(value: unknown): void {
  do_not_optimize(value);
}

export const oneShot: BenchGroup = {
  name: `registering ${WAITERS} one-shot listeners then notifying`,
  cases: [
    {
      name: "CallbackNext",
      setup() {
        const next = new Next();
        const emitter = next.emitter;
        const waiters = Array.from(
          { length: WAITERS },
          () => (value: unknown) => listener(value),
        );

        return () => {
          for (const waiter of waiters) emitter.add(waiter);
          next.emit(1, undefined);
        };
      },
    },
    {
      name: "node EventEmitter once",
      setup() {
        const emitter = new EventEmitter();
        emitter.setMaxListeners(0);
        const waiters = Array.from(
          { length: WAITERS },
          () => (value: unknown) => listener(value),
        );

        return () => {
          for (const waiter of waiters) emitter.once("value", waiter);
          emitter.emit("value", 1);
        };
      },
    },
    {
      name: "array swap",
      setup() {
        let pending: Array<(value: unknown) => void> = [];
        const waiters = Array.from(
          { length: WAITERS },
          () => (value: unknown) => listener(value),
        );

        return () => {
          for (const waiter of waiters) pending.push(waiter);
          const ready = pending;
          pending = [];
          for (const fn of ready) fn(1);
        };
      },
    },
  ],
};
