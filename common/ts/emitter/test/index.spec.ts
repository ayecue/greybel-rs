import { describe, expect, it } from "vitest";
import { Emitter } from "../src/emitter.js";
import * as emitter from "../src/index.js";
import { NextEmitter } from "../src/next.js";

describe("emitter", () => {
  it("should expose exactly the public runtime surface", () => {
    const expected = {
      Emitter,
      NextEmitter,
    };

    const exported = { ...emitter };

    expect(exported).toEqual(expected);
  });
});
