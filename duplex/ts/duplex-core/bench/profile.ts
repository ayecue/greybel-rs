import { writeFileSync } from 'node:fs';
import { Session } from 'node:inspector/promises';

import { groups } from './groups.js';

const [filter, rounds = '2000'] = process.argv.slice(2);
const selected = groups
  .flatMap((group) => group.cases.map((entry) => ({ label: `${group.name} / ${entry.name}`, entry })))
  .filter(({ label }) => label.toLowerCase().includes((filter ?? '').toLowerCase()));

if (selected.length !== 1) throw new Error(`select one case of: ${selected.map(({ label }) => label).join(', ')}`);

const { label, entry } = selected[0]!;
const iteration = await entry.setup();

for (let round = 0; round < Number(rounds) / 10; round++) await iteration();

const session = new Session();

session.connect();
await session.post('Profiler.enable');
await session.post('Profiler.setSamplingInterval', { interval: 50 });
await session.post('Profiler.start');

const started = performance.now();

for (let round = 0; round < Number(rounds); round++) await iteration();

const elapsed = performance.now() - started;
const { profile } = await session.post('Profiler.stop');

writeFileSync(process.env.OUT ?? 'bench.cpuprofile', JSON.stringify(profile));

const self = new Map<string, number>();
const total = profile.samples!.length;
const byId = new Map(profile.nodes.map((node) => [node.id, node]));

for (const id of profile.samples!) {
  const frame = byId.get(id)!.callFrame;
  const name = `${frame.functionName || '(anonymous)'} ${frame.url.split('/').slice(-2).join('/')}:${frame.lineNumber + 1}`;

  self.set(name, (self.get(name) ?? 0) + 1);
}

console.log(`${label}: ${((elapsed * 1000) / Number(rounds)).toFixed(1)} µs/iteration`);
for (const [name, count] of [...self].sort((a, b) => b[1] - a[1]).slice(0, Number(process.env.TOP ?? 35))) {
  console.log(`${((count / total) * 100).toFixed(1).padStart(5)}%  ${name}`);
}
