import { afterEach, describe, expect, it, vi } from "vitest";
import { provider, reporter } from "../../src/host/node.js";
import { has, invoke } from "../../src/utils/methods.js";
import { captureTasks, thrownBy } from "../support/capture-tasks.js";
import { fakeProcess } from "../support/fake-process.js";
import { withGlobal } from "../support/globals.js";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("node host", () => {
  describe("reporter", () => {
    it("should carry a registered handler", () => {
      const target = reporter;

      const registered = has(target);

      expect(registered).toBe(true);
    });
  });

  describe("with process.emit", () => {
    it("should emit uncaughtExceptionMonitor and then uncaughtException with the error and its origin", () => {
      const fake = fakeProcess();
      const host = provider();
      const failure = new Error("boom");

      const reported = withGlobal("process", fake.process, () =>
        invoke(host, failure),
      );

      expect(reported).toBe(true);
      expect(fake.emitted).toEqual([
        ["uncaughtExceptionMonitor", failure, "uncaughtException"],
        ["uncaughtException", failure, "uncaughtException"],
      ]);
    });

    it("should dispatch to the listeners before the caller continues", () => {
      const order: Array<string | symbol> = [];
      const fake = fakeProcess((event) => order.push(event));
      const host = provider();

      withGlobal("process", fake.process, () => {
        invoke(host, new Error("boom"));
        order.push("caller");
      });

      expect(order).toEqual([
        "uncaughtExceptionMonitor",
        "uncaughtException",
        "caller",
      ]);
    });

    it("should not schedule a rethrow when the listeners took the error", () => {
      const fake = fakeProcess();
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();

      withGlobal("process", fake.process, () =>
        invoke(host, new Error("boom")),
      );

      expect(tasks).toEqual([]);
    });

    it("should not throw when an uncaughtException listener throws", () => {
      const fake = fakeProcess((event) => {
        if (event === "uncaughtException") throw new Error("listener broke");
      });
      const { scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();

      const act = (): boolean =>
        withGlobal("process", fake.process, () =>
          invoke(host, new Error("boom")),
        );

      expect(act).not.toThrow();
    });

    it("should rethrow the error from a microtask when a listener throws", () => {
      const fake = fakeProcess((event) => {
        if (event === "uncaughtException") throw new Error("listener broke");
      });
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();
      const failure = new Error("boom");

      withGlobal("process", fake.process, () => invoke(host, failure));

      expect(tasks.map(thrownBy)).toEqual([failure]);
    });

    it("should still emit uncaughtException when a monitor listener throws", () => {
      const fake = fakeProcess((event) => {
        if (event === "uncaughtExceptionMonitor") {
          throw new Error("monitor broke");
        }
      });
      const { scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();

      withGlobal("process", fake.process, () =>
        invoke(host, new Error("boom")),
      );

      expect(fake.emitted.map(([event]) => event)).toEqual([
        "uncaughtExceptionMonitor",
        "uncaughtException",
      ]);
    });
  });

  describe("without a process", () => {
    it("should decline the error", () => {
      const host = provider();

      const reported = withGlobal("process", undefined, () =>
        invoke(host, new Error("boom")),
      );

      expect(reported).toBe(false);
    });

    it("should unregister its handler once it declined", () => {
      const host = provider();

      withGlobal("process", undefined, () => invoke(host, new Error("boom")));

      expect(has(host)).toBe(false);
    });
  });

  describe("with a process shim that has no emit", () => {
    it("should decline the error", () => {
      const host = provider();

      const reported = withGlobal("process", {}, () =>
        invoke(host, new Error("boom")),
      );

      expect(reported).toBe(false);
    });

    it("should keep declining after process.emit appears", () => {
      const host = provider();
      withGlobal("process", {}, () => invoke(host, new Error("first")));
      const fake = fakeProcess();

      const reported = withGlobal("process", fake.process, () =>
        invoke(host, new Error("second")),
      );

      expect(reported).toBe(false);
      expect(fake.emitted).toEqual([]);
    });
  });
});
