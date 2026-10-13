import { describe, expect, it, vi } from "vitest";
import type { Ctx } from "../../src/types.js";
import { add, has, invoke, remove } from "../../src/utils/methods.js";
import { primitives } from "../support/methods.js";

describe("add", () => {
  it("should register the handler on an object", () => {
    const target = {};

    add(target, () => true);

    expect(has(target)).toBe(true);
  });

  it("should register the handler on a function", () => {
    const target = (): void => {};

    add(target, () => true);

    expect(has(target)).toBe(true);
  });

  it("should replace a handler that is already registered", () => {
    const target = {};
    const first = vi.fn(() => true);
    const second = vi.fn(() => true);
    add(target, first);

    add(target, second);

    invoke(target, new Error("boom"));
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledOnce();
  });

  it("should not carry the handler over when the target is spread", () => {
    const target = {};
    add(target, () => true);

    const copy = { ...target };

    expect(has(copy)).toBe(false);
  });

  it.each(primitives)("should reject %s as target", (_label, target) => {
    const handler = (): boolean => true;

    const act = (): void => add(target, handler);

    expect(act).toThrow(TypeError);
  });
});

describe("invoke", () => {
  it("should hand the error to the registered handler", () => {
    const target = {};
    const handler = vi.fn(() => true);
    add(target, handler);
    const failure = new Error("boom");

    invoke(target, failure);

    expect(handler).toHaveBeenCalledExactlyOnceWith(failure);
  });

  it("should call the handler with the target as this", () => {
    const target = {};
    let receiver: unknown;
    add(target, function (this: Ctx): boolean {
      receiver = this;
      return true;
    });

    invoke(target, new Error("boom"));

    expect(receiver).toBe(target);
  });

  it("should return what the handler returns", () => {
    const target = {};
    add(target, () => false);

    const handled = invoke(target, new Error("boom"));

    expect(handled).toBe(false);
  });

  it("should return false when no handler is registered", () => {
    const target = {};

    const handled = invoke(target, new Error("boom"));

    expect(handled).toBe(false);
  });

  it.each(
    primitives,
  )("should return false for %s as target", (_label, target) => {
    const failure = new Error("boom");

    const handled = invoke(target, failure);

    expect(handled).toBe(false);
  });

  it("should propagate an error thrown by the handler", () => {
    const target = {};
    const handlerFailure = new Error("handler broke");
    add(target, () => {
      throw handlerFailure;
    });

    const act = (): boolean => invoke(target, new Error("boom"));

    expect(act).toThrow(handlerFailure);
  });
});

describe("remove", () => {
  it("should stop the handler from being invoked", () => {
    const target = {};
    const handler = vi.fn(() => true);
    add(target, handler);

    remove(target);

    expect(invoke(target, new Error("boom"))).toBe(false);
    expect(handler).not.toHaveBeenCalled();
  });

  it("should leave no property behind on the target", () => {
    const target = {};
    add(target, () => true);

    remove(target);

    expect(Object.getOwnPropertySymbols(target)).toEqual([]);
  });

  it("should not throw when no handler is registered", () => {
    const target = {};

    const act = (): void => remove(target);

    expect(act).not.toThrow();
  });

  it.each(primitives)("should reject %s as target", (_label, target) => {
    const subject = target;

    const act = (): void => remove(subject);

    expect(act).toThrow(TypeError);
  });
});

describe("has", () => {
  it("should be false before a handler is registered", () => {
    const target = {};

    const registered = has(target);

    expect(registered).toBe(false);
  });

  it("should be true once a handler is registered", () => {
    const target = {};
    add(target, () => true);

    const registered = has(target);

    expect(registered).toBe(true);
  });

  it.each(primitives)("should be false for %s as target", (_label, target) => {
    const subject = target;

    const registered = has(subject);

    expect(registered).toBe(false);
  });
});
