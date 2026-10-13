import { handleError } from "handle-error";
import { afterEach, describe, expect, expectTypeOf, it, vi } from "vitest";
import { NextEmitter } from "../../src/next.js";
import type { Callback } from "../../src/types.js";

vi.mock("handle-error", () => ({ handleError: vi.fn() }));

afterEach(() => {
  vi.mocked(handleError).mockClear();
});

describe("NextEmitter", () => {
  describe("emitter", () => {
    it("should keep returning the same consumer side", () => {
      const next = new NextEmitter();

      const first = next.emitter;
      const second = next.emitter;

      expect(second).toBe(first);
    });

    it("should hand out a fresh consumer side after being disposed", () => {
      const next = new NextEmitter();
      const before = next.emitter;
      before.add(() => {});

      next.dispose();
      const after = next.emitter;

      expect(after).not.toBe(before);
      expect(after.size).toBe(0);
    });
  });

  describe("membership", () => {
    it("should start empty", () => {
      const next = new NextEmitter();

      const size = next.emitter.size;

      expect(size).toBe(0);
    });

    it("should register the same callback twice as two independent registrations", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();

      next.emitter.add(listener);
      next.emitter.add(listener);
      const size = next.emitter.size;
      next.emit();

      expect(size).toBe(2);
      expect(listener).toHaveBeenCalledTimes(2);
    });

    it("should shrink its size when a registration is disposed", () => {
      const next = new NextEmitter();
      next.emitter.add(() => {});
      const dispose = next.emitter.addDisposable(() => {});

      dispose();

      expect(next.emitter.size).toBe(1);
    });

    it("should be empty again once it has emitted", () => {
      const next = new NextEmitter();
      next.emitter.add(() => {});
      next.emitter.add(() => {});

      next.emit();

      expect(next.emitter.size).toBe(0);
    });

    it("should drop every callback when disposed", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();
      next.emitter.add(listener);

      next.dispose();
      next.emit();

      expect(listener).not.toHaveBeenCalled();
      expect(next.emitter.size).toBe(0);
    });
  });

  describe("disposables", () => {
    it("should remove the callback when the disposable is called", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();
      next.emitter.add(() => {});
      const dispose = next.emitter.addDisposable(listener);

      dispose();
      next.emit();

      expect(listener).not.toHaveBeenCalled();
    });

    it("should remove only its own registration when the same callback was added twice", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();
      const disposeFirst = next.emitter.addDisposable(listener);
      next.emitter.addDisposable(listener);

      disposeFirst();
      next.emit();

      expect(listener).toHaveBeenCalledOnce();
    });

    it("should remove its registration only once when called twice", () => {
      const next = new NextEmitter();
      next.emitter.add(() => {});
      next.emitter.add(() => {});
      const dispose = next.emitter.addDisposable(() => {});
      dispose();

      dispose();

      expect(next.emitter.size).toBe(2);
    });

    it("should not remove a registration made after it emitted", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();
      const dispose = next.emitter.addDisposable(listener);
      next.emit();
      next.emitter.add(listener);

      dispose();
      next.emit();

      expect(listener).toHaveBeenCalledTimes(2);
    });

    it("should skip a callback disposed during the same emit", () => {
      const next = new NextEmitter();
      const received: string[] = [];
      let disposeSecond: () => void = () => {};
      next.emitter.add(() => {
        received.push("first");
        disposeSecond();
      });
      disposeSecond = next.emitter.addDisposable(() => {
        received.push("second");
      });
      next.emitter.add(() => {
        received.push("third");
      });

      next.emit();

      expect(received).toEqual(["first", "third"]);
    });
  });

  describe("emit", () => {
    it("should do nothing when no callback was ever added", () => {
      const next = new NextEmitter();

      const act = (): void => next.emit();

      expect(act).not.toThrow();
    });

    it("should pass the arguments to every callback in insertion order", () => {
      const next = new NextEmitter<(error: string, value: number) => void>();
      const received: unknown[] = [];
      next.emitter.add((error, value) => received.push(["a", error, value]));
      next.emitter.add((error, value) => received.push(["b", error, value]));

      next.emit("error", 7);

      expect(received).toEqual([
        ["a", "error", 7],
        ["b", "error", 7],
      ]);
    });

    it("should pass every argument it is emitted with, however many there are", () => {
      const next = new NextEmitter<
        (a: number, b: string, c: boolean, d: null) => void
      >();
      const listener =
        vi.fn<(a: number, b: string, c: boolean, d: null) => void>();
      next.emitter.add(listener);

      next.emit(1, "two", true, null);

      expect(listener).toHaveBeenCalledExactlyOnceWith(1, "two", true, null);
    });

    it("should take its emit parameters from the callback type", () => {
      const next = new NextEmitter<
        (error: Error | undefined, value: number) => void
      >();

      const emit = next.emit.bind(next);

      expectTypeOf(emit).parameters.toEqualTypeOf<
        [error: Error | undefined, value: number]
      >();
    });

    it("should call each callback only for the next emit", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();
      next.emitter.add(listener);

      next.emit();
      next.emit();

      expect(listener).toHaveBeenCalledOnce();
    });

    it("should defer a callback added during an emit to the following emit", () => {
      const next = new NextEmitter();
      const late = vi.fn<Callback>();
      next.emitter.add(() => {
        next.emitter.add(late);
      });
      next.emit();
      const callsAfterFirst = late.mock.calls.length;

      next.emit();

      expect(callsAfterFirst).toBe(0);
      expect(late).toHaveBeenCalledOnce();
    });

    it("should finish the current emit when disposed during it", () => {
      const next = new NextEmitter();
      const received: string[] = [];
      next.emitter.add(() => {
        received.push("a");
        next.dispose();
      });
      next.emitter.add(() => {
        received.push("b");
      });

      next.emit();

      expect(received).toEqual(["a", "b"]);
    });

    it("should report an error and finish the outer emit when a callback emits again", () => {
      const next = new NextEmitter();
      const received: string[] = [];
      next.emitter.add(() => {
        received.push("a");
        next.emit();
      });
      next.emitter.add(() => {
        received.push("b");
      });

      next.emit();

      expect(received).toEqual(["a", "b"]);
      expect(handleError).toHaveBeenCalledExactlyOnceWith(
        new Error("Cannot emit while processing."),
        next,
      );
    });

    it("should accept a new emit once the previous one has finished", () => {
      const next = new NextEmitter();
      const listener = vi.fn<Callback>();
      next.emitter.add(() => {
        next.emitter.add(listener);
      });
      next.emit();

      next.emit();

      expect(listener).toHaveBeenCalledOnce();
      expect(handleError).not.toHaveBeenCalled();
    });

    it("should keep calling the remaining callbacks after one throws", () => {
      const next = new NextEmitter<(value: number) => void>();
      const failure = new Error("boom");
      const survivor = vi.fn<(value: number) => void>();
      next.emitter.add(() => {
        throw failure;
      });
      next.emitter.add(survivor);

      next.emit(3);

      expect(survivor).toHaveBeenCalledExactlyOnceWith(3);
      expect(handleError).toHaveBeenCalledExactlyOnceWith(failure, next);
    });

    it("should report every error when several callbacks throw", () => {
      const next = new NextEmitter();
      const first = new Error("first");
      const second = new Error("second");
      const survivor = vi.fn<Callback>();
      next.emitter.add(() => {
        throw first;
      });
      next.emitter.add(survivor);
      next.emitter.add(() => {
        throw second;
      });

      next.emit();

      expect(vi.mocked(handleError).mock.calls).toEqual([[first, next], [second, next]]);
      expect(survivor).toHaveBeenCalledOnce();
    });
  });
});
