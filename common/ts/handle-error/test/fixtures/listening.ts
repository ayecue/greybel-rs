import { writeSync } from "node:fs";
import { handleError } from "../../src/index.js";

const events: string[] = [];

process.on("uncaughtExceptionMonitor", (error, origin) => {
  events.push(`monitor:${error.message}:${origin}`);
});
process.on("uncaughtException", (error, origin) => {
  events.push(`uncaught:${error.message}:${origin}`);
});

handleError(new Error("first"));
events.push("after-first");
handleError(new Error("second"));
events.push("after-second");

setTimeout(() => {
  writeSync(1, JSON.stringify(events));
}, 10);
