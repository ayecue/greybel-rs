import type { BenchGroup } from "./case.js";
import { oneShot } from "./cases/next.js";
import { churn, notifyOne, notifySixteen } from "./cases/notifier.js";

export const groups: readonly BenchGroup[] = [
  notifyOne,
  notifySixteen,
  churn,
  oneShot,
];
