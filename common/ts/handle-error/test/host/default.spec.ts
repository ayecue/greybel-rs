import { afterEach, describe, expect, it, vi } from "vitest";
import { provider, reporter } from "../../src/host/default.js";
import { has, invoke } from "../../src/utils/methods.js";
import { captureTasks, thrownBy } from "../support/capture-tasks.js";
import { withGlobal } from "../support/globals.js";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("default host", () => {
  describe("reporter", () => {
    it("should carry a registered handler", () => {
      const target = reporter;

      const registered = has(target);

      expect(registered).toBe(true);
    });
  });

  describe("provider", () => {
    it("should build a new reporter on every call", () => {
      const first = provider();

      const second = provider();

      expect(second).not.toBe(first);
      expect(has(second)).toBe(true);
    });
  });

  describe("with queueMicrotask", () => {
    it("should accept the error", () => {
      const { scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();

      const reported = invoke(host, new Error("boom"));

      expect(reported).toBe(true);
    });

    it("should rethrow the error from a microtask", () => {
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();
      const failure = new Error("boom");

      invoke(host, failure);

      expect(tasks.map(thrownBy)).toEqual([failure]);
    });

    it("should not throw before the microtask runs", () => {
      const { scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();

      const act = (): boolean => invoke(host, new Error("boom"));

      expect(act).not.toThrow();
    });

    it("should rethrow the same non-error value", () => {
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const host = provider();
      const failure = "plain string";

      invoke(host, failure);

      expect(thrownBy(tasks[0])).toBe(failure);
    });
  });

  describe("without queueMicrotask", () => {
    it("should rethrow the error from process.nextTick", () => {
      vi.stubGlobal("queueMicrotask", undefined);
      const { tasks, scheduler } = captureTasks();
      const host = provider();
      const failure = new Error("boom");

      const reported = withGlobal("process", { nextTick: scheduler }, () =>
        invoke(host, failure),
      );

      expect(reported).toBe(true);
      expect(tasks.map(thrownBy)).toEqual([failure]);
    });

    it("should keep the mechanism it chose on its first report", () => {
      vi.stubGlobal("queueMicrotask", undefined);
      const nextTick = captureTasks();
      const microtasks = captureTasks();
      const host = provider();
      withGlobal("process", { nextTick: nextTick.scheduler }, () =>
        invoke(host, new Error("first")),
      );
      vi.stubGlobal("queueMicrotask", microtasks.scheduler);
      const failure = new Error("second");

      withGlobal("process", { nextTick: nextTick.scheduler }, () =>
        invoke(host, failure),
      );

      expect(microtasks.tasks).toEqual([]);
      expect(thrownBy(nextTick.tasks[1])).toBe(failure);
    });
  });

  describe("without queueMicrotask and process.nextTick", () => {
    it("should rethrow the error from a timer", () => {
      vi.stubGlobal("queueMicrotask", undefined);
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("setTimeout", scheduler);
      const host = provider();
      const failure = new Error("boom");

      const reported = withGlobal("process", undefined, () =>
        invoke(host, failure),
      );

      expect(reported).toBe(true);
      expect(tasks.map(thrownBy)).toEqual([failure]);
    });
  });
});
