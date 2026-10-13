import { writeSync } from "node:fs";
import { handleError } from "../../src/index.js";

handleError(new Error("boom"));


writeSync(1, "after\n");
