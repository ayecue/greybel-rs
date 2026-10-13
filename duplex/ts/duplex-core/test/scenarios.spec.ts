import { describe, expect, it } from 'vitest';

import { Scenario, scenarios, validate } from './support/scenario.js';

describe('the shared scenarios', () => {
  it('should be there', () => {
    expect(scenarios.length).toBeGreaterThan(0);
  });

  it.each(scenarios)('should describe %s as the schema says', (_name, scenario) => {
    const valid = validate(scenario);

    expect(validate.errors ?? []).toEqual([]);
    expect(valid).toBe(true);
  });

  it.each(scenarios)('should pass %s', async (_name, scenario) => {
    await Scenario.run(scenario);
  });
});
