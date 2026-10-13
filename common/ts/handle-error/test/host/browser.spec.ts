import { afterEach, describe, expect, it, vi } from "vitest";
import { provider, reporter } from "../../src/host/browser.js";
import { has, invoke } from "../../src/utils/methods.js";
import { captureTasks, thrownBy } from "../support/capture-tasks.js";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("browser host", () => {
  describe("reporter", () => {
    it("should carry a registered handler", () => {
      const target = reporter;

      const registered = has(target);

      expect(registered).toBe(true);
    });
  });

  describe("without reportError", () => {
    it("should decline the error", () => {
      const host = provider();

      const reported = invoke(host, new Error("boom"));

      expect(reported).toBe(false);
    });

    it("should unregister its handler once it declined", () => {
      const host = provider();

      invoke(host, new Error("boom"));

      expect(has(host)).toBe(false);
    });

    it("should decline when reportError is not a function", () => {
      vi.stubGlobal("reportError", "not a function");
      const host = provider();

      const reported = invoke(host, new Error("boom"));

      expect(reported).toBe(false);
    });

    it("should keep declining after reportError appears", () => {
      const host = provider();
      invoke(host, new Error("first"));
      const reportError = vi.fn();
      vi.stubGlobal("reportError", reportError);

      const reported = invoke(host, new Error("second"));

      expect(reported).toBe(false);
      expect(reportError).not.toHaveBeenCalled();
    });
  });

  describe("with reportError", () => {
    it("should hand the error to reportError and accept it", () => {
      const reportError = vi.fn();
      vi.stubGlobal("reportError", reportError);
      const host = provider();
      const failure = new Error("boom");

      const reported = invoke(host, failure);

      expect(reported).toBe(true);
      expect(reportError).toHaveBeenCalledExactlyOnceWith(failure);
    });

    it("should call reportError with the global object as this", () => {
      let receiver: unknown;
      vi.stubGlobal("reportError", function (this: unknown) {
        receiver = this;
      });
      const host = provider();

      invoke(host, new Error("boom"));

      expect(receiver).toBe(globalThis);
    });

    it("should not throw when reportError throws", () => {
      const { scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      vi.stubGlobal("reportError", () => {
        throw new Error("reporter broke");
      });
      const host = provider();

      const act = (): boolean => invoke(host, new Error("boom"));

      expect(act).not.toThrow();
    });

    it("should rethrow the error from a microtask when reportError throws", () => {
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      vi.stubGlobal("reportError", () => {
        throw new Error("reporter broke");
      });
      const host = provider();
      const failure = new Error("boom");

      const reported = invoke(host, failure);

      expect(reported).toBe(true);
      expect(tasks.map(thrownBy)).toEqual([failure]);
    });
  });
});
