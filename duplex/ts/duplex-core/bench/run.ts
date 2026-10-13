import { bench, group, run, summary } from 'mitata';

import { groups } from './groups.js';

const filters = process.argv.slice(2).map((filter) => filter.toLowerCase());

function include(label: string): boolean {
  return filters.length === 0 || filters.some((filter) => label.toLowerCase().includes(filter));
}

for (const { name, cases } of groups) {
  const selected = cases.filter((entry) => include(`${name} / ${entry.name}`));

  if (selected.length === 0) continue;

  const iterations = await Promise.all(selected.map((entry) => entry.setup()));

  group(name, () => {
    summary(() => {
      selected.forEach((entry, index) => bench(entry.name, iterations[index]!));
    });
  });
}

await run();
