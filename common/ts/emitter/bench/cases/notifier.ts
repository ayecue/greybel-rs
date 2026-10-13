import { EventEmitter } from "node:events";
import { do_not_optimize } from "mitata";
import { Subject } from "rxjs";
import { Emitter } from "../../src/index.js";
import type { BenchGroup } from "../case.js";

function listener(value: unknown): void {
  do_not_optimize(value);
}

function notifyTo(count: number): BenchGroup {
  return {
    name: `notifying ${count} listener${count === 1 ? "" : "s"}`,
    cases: [
      {
        name: "CallbackNotifier",
        setup() {
          const notifier = new Emitter();
          const emitter = notifier.emitter;
          for (let i = 0; i < count; i++)
            emitter.add((value) => listener(value));

          return () => {
            notifier.emit(1, undefined);
          };
        },
      },
      {
        name: "rxjs Subject",
        setup() {
          const subject = new Subject<number>();
          for (let i = 0; i < count; i++)
            subject.subscribe((value) => listener(value));

          return () => {
            subject.next(1);
          };
        },
      },
      {
        name: "node EventEmitter",
        setup() {
          const emitter = new EventEmitter();
          emitter.setMaxListeners(0);
          for (let i = 0; i < count; i++)
            emitter.on("value", (value) => listener(value));

          return () => {
            emitter.emit("value", 1);
          };
        },
      },
      {
        name: "Set of functions",
        setup() {
          const set = new Set<(value: number) => void>();
          for (let i = 0; i < count; i++) set.add((value) => listener(value));

          return () => {
            for (const fn of set) fn(1);
          };
        },
      },
    ],
  };
}

export const notifyOne = notifyTo(1);
export const notifySixteen = notifyTo(16);

const RESIDENTS = 16;

export const churn: BenchGroup = {
  name: `adding then removing one listener beside ${RESIDENTS} others`,
  cases: [
    {
      name: "CallbackNotifier addDisposable",
      setup() {
        const notifier = new Emitter();
        const emitter = notifier.emitter;
        for (let i = 0; i < RESIDENTS; i++) emitter.add(() => {});

        return () => {
          emitter.addDisposable(() => {})();
        };
      },
    },
    {
      name: "rxjs Subject",
      setup() {
        const subject = new Subject<number>();
        for (let i = 0; i < RESIDENTS; i++) subject.subscribe(() => {});

        return () => {
          subject.subscribe(() => {}).unsubscribe();
        };
      },
    },
    {
      name: "node EventEmitter",
      setup() {
        const emitter = new EventEmitter();
        emitter.setMaxListeners(0);
        for (let i = 0; i < RESIDENTS; i++) emitter.on("value", () => {});

        return () => {
          const fn = (): void => {};
          emitter.on("value", fn);
          emitter.off("value", fn);
        };
      },
    },
  ],
};
