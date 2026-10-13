import { afterEach, describe, expect, it, vi } from "vitest";
import { captureTasks, thrownBy } from "./support/capture-tasks.js";
import { fakeProcess } from "./support/fake-process.js";
import { withGlobal } from "./support/globals.js";
import { cleanups, freshHandleError, runFixture } from "./support/handle-error.js";

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  vi.unstubAllGlobals();
});

describe("handleError", () => {
  describe("with a context handler", () => {
    it("should hand the error to the context handler", async () => {
      const handleError = await freshHandleError();
      const context = {};
      const handler = vi.fn(() => true);
      handleError.add(context, handler);
      const failure = new Error("boom");

      handleError(failure, context);

      expect(handler).toHaveBeenCalledExactlyOnceWith(failure);
    });

    it("should not reach the host reporters once the context handler took the error", async () => {
      const reportError = vi.fn();
      vi.stubGlobal("reportError", reportError);
      const handleError = await freshHandleError();
      const context = {};
      handleError.add(context, () => true);
      const fake = fakeProcess();

      withGlobal("process", fake.process, () =>
        handleError(new Error("boom"), context),
      );

      expect(reportError).not.toHaveBeenCalled();
      expect(fake.emitted).toEqual([]);
    });

    it("should fall through to the host reporters when the context handler declines", async () => {
      const reportError = vi.fn();
      vi.stubGlobal("reportError", reportError);
      const handleError = await freshHandleError();
      const context = {};
      handleError.add(context, () => false);
      const failure = new Error("boom");

      handleError(failure, context);

      expect(reportError).toHaveBeenCalledExactlyOnceWith(failure);
    });

    it("should no longer reach a handler once it was removed", async () => {
      vi.stubGlobal("reportError", vi.fn());
      const handleError = await freshHandleError();
      const context = {};
      const handler = vi.fn(() => true);
      handleError.add(context, handler);
      handleError.remove(context);

      handleError(new Error("boom"), context);

      expect(handler).not.toHaveBeenCalled();
    });

    it("should propagate an error thrown by the context handler", async () => {
      const handleError = await freshHandleError();
      const context = {};
      const handlerFailure = new Error("handler broke");
      handleError.add(context, () => {
        throw handlerFailure;
      });

      const act = (): void => handleError(new Error("boom"), context);

      expect(act).toThrow(handlerFailure);
    });

    it("should ignore a context that cannot carry a handler", async () => {
      const reportError = vi.fn();
      vi.stubGlobal("reportError", reportError);
      const handleError = await freshHandleError();
      const failure = new Error("boom");

      handleError(failure, "not a context");

      expect(reportError).toHaveBeenCalledExactlyOnceWith(failure);
    });
  });

  describe("without a context", () => {
    it("should use a handler registered on the global object", async () => {
      const handleError = await freshHandleError();
      const handler = vi.fn(() => true);
      handleError.add(globalThis, handler);
      cleanups.push(() => handleError.remove(globalThis));
      const failure = new Error("boom");

      handleError(failure);

      expect(handler).toHaveBeenCalledExactlyOnceWith(failure);
    });
  });

  describe("with reportError", () => {
    it("should hand the error to reportError", async () => {
      const reportError = vi.fn();
      vi.stubGlobal("reportError", reportError);
      const handleError = await freshHandleError();
      const failure = new Error("boom");

      handleError(failure);

      expect(reportError).toHaveBeenCalledExactlyOnceWith(failure);
    });

    it("should not emit on the process once reportError took the error", async () => {
      vi.stubGlobal("reportError", vi.fn());
      const handleError = await freshHandleError();
      const fake = fakeProcess();

      withGlobal("process", fake.process, () => handleError(new Error("boom")));

      expect(fake.emitted).toEqual([]);
    });

    it("should rethrow the error from a microtask when reportError throws", async () => {
      vi.stubGlobal("reportError", () => {
        throw new Error("reporter broke");
      });
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const handleError = await freshHandleError();
      const failure = new Error("boom");

      handleError(failure);

      expect(tasks.map(thrownBy)).toEqual([failure]);
    });
  });

  describe("without reportError", () => {
    it("should emit the error on the process", async () => {
      const handleError = await freshHandleError();
      const fake = fakeProcess();
      const failure = new Error("boom");

      withGlobal("process", fake.process, () => handleError(failure));

      expect(fake.emitted).toEqual([
        ["uncaughtExceptionMonitor", failure, "uncaughtException"],
        ["uncaughtException", failure, "uncaughtException"],
      ]);
    });

    it("should not throw when a process listener throws", async () => {
      const handleError = await freshHandleError();
      const fake = fakeProcess(() => {
        throw new Error("listener broke");
      });
      const { scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);

      const act = (): void =>
        withGlobal("process", fake.process, () =>
          handleError(new Error("boom")),
        );

      expect(act).not.toThrow();
    });
  });

  describe("without reportError and process.emit", () => {
    it("should rethrow the error from a microtask", async () => {
      const { tasks, scheduler } = captureTasks();
      vi.stubGlobal("queueMicrotask", scheduler);
      const handleError = await freshHandleError();
      const failure = new Error("boom");

      withGlobal("process", {}, () => handleError(failure));

      expect(tasks.map(thrownBy)).toEqual([failure]);
    });
  });

  describe("in a real Node process", () => {
    it("should deliver each error to the uncaught listeners before the caller continues", () => {
      const fixture = "listening.ts";

      const result = runFixture(fixture);

      expect(result.status).toBe(0);
      expect(JSON.parse(String(result.stdout))).toEqual([
        "monitor:first:uncaughtException",
        "uncaught:first:uncaughtException",
        "after-first",
        "monitor:second:uncaughtException",
        "uncaught:second:uncaughtException",
        "after-second",
      ]);
    });

    it("should let Node print the error and exit with code 1 once the caller finished when nothing listens", () => {
      const fixture = "unlistened.ts";

      const result = runFixture(fixture);

      expect(String(result.stdout)).toBe("after\n");
      expect(String(result.stderr)).toContain("Error: boom");
      expect(result.status).toBe(1);
    });
  });
});
