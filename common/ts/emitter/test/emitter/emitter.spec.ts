import { handleError } from "handle-error";
import { afterEach, describe, expect, expectTypeOf, it, vi } from "vitest";
import { Emitter } from "../../src/emitter.js";
import type { Callback } from "../../src/types.js";
import { slotCount } from "../support/slots.js";

vi.mock("handle-error", () => ({ handleError: vi.fn() }));

afterEach(() => {
  vi.mocked(handleError).mockClear();
});

describe("Emitter", () => {
  describe("emitter", () => {
    it("should keep returning the same consumer side", () => {
      const emitter = new Emitter();

      const first = emitter.emitter;
      const second = emitter.emitter;

      expect(second).toBe(first);
    });

    it("should hand out a fresh consumer side after being disposed", () => {
      const emitter = new Emitter();
      const before = emitter.emitter;
      before.add(() => {});

      emitter.dispose();
      const after = emitter.emitter;

      expect(after).not.toBe(before);
      expect(after.size).toBe(0);
    });
  });

  describe("membership", () => {
    it("should start inactive and empty", () => {
      const emitter = new Emitter();

      const state = {
        isActive: emitter.emitter.isActive,
        size: emitter.emitter.size,
      };

      expect(state).toEqual({ isActive: false, size: 0 });
    });

    it("should become active when the first callback is added", () => {
      const emitter = new Emitter();

      emitter.emitter.add(() => {});

      expect(emitter.emitter.isActive).toBe(true);
      expect(emitter.emitter.size).toBe(1);
    });

    it("should register the same callback twice as two independent registrations", () => {
      const emitter = new Emitter();
      const listener = vi.fn<Callback>();

      emitter.emitter.add(listener);
      emitter.emitter.add(listener);
      emitter.emit();

      expect(emitter.emitter.size).toBe(2);
      expect(listener).toHaveBeenCalledTimes(2);
    });

    it("should shrink its size when a registration is disposed", () => {
      const emitter = new Emitter();
      emitter.emitter.add(() => {});
      const dispose = emitter.emitter.addDisposable(() => {});

      dispose();

      expect(emitter.emitter.size).toBe(1);
      expect(emitter.emitter.isActive).toBe(true);
    });

    it("should become inactive when its last registration is disposed", () => {
      const emitter = new Emitter();
      const dispose = emitter.emitter.addDisposable(() => {});

      dispose();

      expect(emitter.emitter.isActive).toBe(false);
      expect(emitter.emitter.size).toBe(0);
    });

    it("should become inactive when the consumer side is disposed", () => {
      const emitter = new Emitter();
      emitter.emitter.add(() => {});
      emitter.emitter.add(() => {});

      emitter.emitter.dispose();

      expect(emitter.emitter.isActive).toBe(false);
      expect(emitter.emitter.size).toBe(0);
    });

    it("should become active again when a callback is added after the consumer side was disposed", () => {
      const emitter = new Emitter();
      emitter.emitter.add(() => {});
      emitter.emitter.dispose();

      emitter.emitter.add(() => {});

      expect(emitter.emitter.isActive).toBe(true);
      expect(emitter.emitter.size).toBe(1);
    });
  });

  describe("disposables", () => {
    it("should remove the callback when the disposable is called", () => {
      const emitter = new Emitter();
      const listener = vi.fn<Callback>();
      emitter.emitter.add(() => {});
      const dispose = emitter.emitter.addDisposable(listener);

      dispose();
      emitter.emit();

      expect(listener).not.toHaveBeenCalled();
    });

    it("should remove only its own registration when the same callback was added twice", () => {
      const emitter = new Emitter();
      const listener = vi.fn<Callback>();
      const disposeFirst = emitter.emitter.addDisposable(listener);
      emitter.emitter.addDisposable(listener);

      disposeFirst();
      emitter.emit();

      expect(emitter.emitter.size).toBe(1);
      expect(listener).toHaveBeenCalledOnce();
    });

    it("should remove its registration only once when called twice", () => {
      const emitter = new Emitter();
      emitter.emitter.add(() => {});
      emitter.emitter.add(() => {});
      const dispose = emitter.emitter.addDisposable(() => {});
      dispose();

      dispose();

      expect(emitter.emitter.size).toBe(2);
    });

    it("should not remove a different callback that backfilled its slot", () => {
      const emitter = new Emitter();
      const replacement = vi.fn<Callback>();
      emitter.emitter.add(() => {});
      const dispose = emitter.emitter.addDisposable(() => {});
      dispose();
      emitter.emitter.add(replacement);

      dispose();
      emitter.emit();

      expect(emitter.emitter.size).toBe(2);
      expect(replacement).toHaveBeenCalledOnce();
    });

    it("should not remove a registration made after the consumer side became inactive and active again", () => {
      const emitter = new Emitter();
      const listener: Callback = () => {};
      const dispose = emitter.emitter.addDisposable(listener);
      dispose();
      emitter.emitter.add(listener);

      dispose();

      expect(emitter.emitter.size).toBe(1);
    });

    it("should ignore a disposable called after the consumer side was disposed", () => {
      const emitter = new Emitter();
      const dispose = emitter.emitter.addDisposable(() => {});
      emitter.emitter.dispose();

      dispose();

      expect(emitter.emitter.isActive).toBe(false);
      expect(emitter.emitter.size).toBe(0);
    });
  });

  describe("emit", () => {
    it("should do nothing when no callback was ever added", () => {
      const emitter = new Emitter();

      const act = (): void => emitter.emit();

      expect(act).not.toThrow();
    });

    it("should pass the arguments to every callback in insertion order", () => {
      const emitter = new Emitter<(error: string, value: number) => void>();
      const received: unknown[] = [];
      emitter.emitter.add((error, value) => received.push(["a", error, value]));
      emitter.emitter.add((error, value) => received.push(["b", error, value]));

      emitter.emit("error", 7);

      expect(received).toEqual([
        ["a", "error", 7],
        ["b", "error", 7],
      ]);
    });

    it("should pass every argument it is emitted with, however many there are", () => {
      const emitter = new Emitter<
        (a: number, b: string, c: boolean, d: null) => void
      >();
      const listener =
        vi.fn<(a: number, b: string, c: boolean, d: null) => void>();
      emitter.emitter.add(listener);

      emitter.emit(1, "two", true, null);

      expect(listener).toHaveBeenCalledExactlyOnceWith(1, "two", true, null);
    });

    it("should take its emit parameters from the callback type", () => {
      const emitter = new Emitter<
        (error: Error | undefined, value: number) => void
      >();

      const emit = emitter.emit.bind(emitter);

      expectTypeOf(emit).parameters.toEqualTypeOf<
        [error: Error | undefined, value: number]
      >();
    });

    it("should keep its callbacks after emitting", () => {
      const emitter = new Emitter();
      const listener = vi.fn<Callback>();
      emitter.emitter.add(listener);

      emitter.emit();
      emitter.emit();

      expect(listener).toHaveBeenCalledTimes(2);
      expect(emitter.emitter.size).toBe(1);
    });

    it("should skip a callback disposed during the same emit", () => {
      const emitter = new Emitter();
      const received: string[] = [];
      let disposeSecond: () => void = () => {};
      emitter.emitter.add(() => {
        received.push("first");
        disposeSecond();
      });
      disposeSecond = emitter.emitter.addDisposable(() => {
        received.push("second");
      });
      emitter.emitter.add(() => {
        received.push("third");
      });

      emitter.emit();

      expect(received).toEqual(["first", "third"]);
    });

    it("should not call a callback appended during the same emit", () => {
      const emitter = new Emitter();
      const late = vi.fn<Callback>();
      emitter.emitter.add(() => {
        emitter.emitter.add(late);
      });

      emitter.emit();

      expect(late).not.toHaveBeenCalled();
      expect(emitter.emitter.size).toBe(2);
    });

    it("should call a callback appended during an emit on the following emit", () => {
      const emitter = new Emitter();
      const late = vi.fn<Callback>();
      let added = false;
      emitter.emitter.add(() => {
        if (added) return;
        added = true;
        emitter.emitter.add(late);
      });
      emitter.emit();

      emitter.emit();

      expect(late).toHaveBeenCalledOnce();
    });

    it("should report an error and finish the outer emit when a callback emits again", () => {
      const emitter = new Emitter();
      const received: string[] = [];
      emitter.emitter.add(() => {
        received.push("a");
        emitter.emit();
      });
      emitter.emitter.add(() => {
        received.push("b");
      });

      emitter.emit();

      expect(received).toEqual(["a", "b"]);
      expect(handleError).toHaveBeenCalledExactlyOnceWith(
        new Error("Cannot emit while processing."),
        emitter,
      );
    });

    it("should accept a new emit once the previous one has finished", () => {
      const emitter = new Emitter();
      const listener = vi.fn<Callback>();
      emitter.emitter.add(listener);
      emitter.emit();

      emitter.emit();

      expect(listener).toHaveBeenCalledTimes(2);
      expect(handleError).not.toHaveBeenCalled();
    });

    it("should stop calling the remaining callbacks when disposed during an emit", () => {
      const emitter = new Emitter();
      const received: string[] = [];
      emitter.emitter.add(() => {
        received.push("a");
        emitter.dispose();
      });
      emitter.emitter.add(() => {
        received.push("b");
      });

      emitter.emit();

      expect(received).toEqual(["a"]);
    });

    it("should not call callbacks added through a consumer side obtained before it was disposed", () => {
      const emitter = new Emitter();
      const listener = vi.fn<Callback>();
      const stale = emitter.emitter;
      emitter.dispose();
      stale.add(listener);

      emitter.emit();

      expect(listener).not.toHaveBeenCalled();
    });

    it("should keep calling the remaining callbacks after one throws", () => {
      const emitter = new Emitter<(value: number) => void>();
      const failure = new Error("boom");
      const survivor = vi.fn<(value: number) => void>();
      emitter.emitter.add(() => {
        throw failure;
      });
      emitter.emitter.add(survivor);

      emitter.emit(3);

      expect(survivor).toHaveBeenCalledExactlyOnceWith(3);
      expect(handleError).toHaveBeenCalledExactlyOnceWith(failure, emitter);
    });

    it("should report every error when several callbacks throw", () => {
      const emitter = new Emitter();
      const first = new Error("first");
      const second = new Error("second");
      const survivor = vi.fn<Callback>();
      emitter.emitter.add(() => {
        throw first;
      });
      emitter.emitter.add(survivor);
      emitter.emitter.add(() => {
        throw second;
      });

      emitter.emit();

      expect(vi.mocked(handleError).mock.calls).toEqual([[first, emitter], [second, emitter]]);
      expect(survivor).toHaveBeenCalledOnce();
    });
  });

  describe("backfill", () => {
    it("should place a new callback into the slot of a disposed one", () => {
      const emitter = new Emitter();
      const received: string[] = [];
      emitter.emitter.add(() => received.push("a"));
      const disposeB = emitter.emitter.addDisposable(() => received.push("b"));
      emitter.emitter.add(() => received.push("c"));
      disposeB();
      emitter.emitter.add(() => received.push("d"));

      emitter.emit();

      expect(received).toEqual(["a", "d", "c"]);
      expect(slotCount(emitter.emitter)).toBe(3);
    });

    it("should keep its slot count at the peak number of registrations under churn", () => {
      const emitter = new Emitter();
      for (let index = 0; index < 3; index++) emitter.emitter.add(() => {});

      for (let cycle = 0; cycle < 100; cycle++) {
        emitter.emitter.addDisposable(() => {})();
      }

      expect(emitter.emitter.size).toBe(3);
      expect(slotCount(emitter.emitter)).toBe(4);
    });
  });
});
