import type { BenchGroup } from './case.js';
import { notifications } from './cases/notifications.js';
import { requests } from './cases/requests.js';
import { streams } from './cases/streams.js';

export const groups: readonly BenchGroup[] = [requests, notifications, streams];
