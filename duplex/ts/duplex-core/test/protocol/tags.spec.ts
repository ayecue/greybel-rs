import { describe, expect, it } from 'vitest';

import { findTag, findAllTags, fieldWithTag } from '../../src/index.js';
import { Calibrate, Kind, Position, Reading, Sensor, Shape, Silence, Unit } from '../sensor.generated.js';

describe('tags', () => {
  it('should retain the tags of the protocol and its messages', () => {
    expect(Sensor.tags).toEqual([{ name: 'origin', value: 'lab' }]);
    expect(Reading.tags).toEqual([
      { name: 'origin', value: 'probe' },
      { name: 'number', value: 34 }
    ]);
    expect(Calibrate.tags).toEqual([{ name: 'number', value: 1000 }]);
    expect(Silence.tags).toEqual([]);
  });

  it("should retain the tags of a message's fields in their order", () => {
    expect(Reading.fields.map((field) => field.name)).toEqual([
      'value',
      'station',
      'labels',
      'position',
      'shape',
      'unit',
      'note'
    ]);
    expect(Reading.fields[0]?.tags).toEqual([
      { name: 'alias', value: 'value' },
      { name: 'alias', value: 'measurement' },
      { name: 'weight', value: 0.5 }
    ]);
    expect(Reading.fields[1]?.tags).toEqual([{ name: 'recipient' }, { name: 'offset', value: -3n }]);
    expect(Reading.fields[2]?.tags).toEqual([{ name: 'sorted', value: true }]);
    expect(fieldWithTag(Reading.fields, 'recipient')?.property).toBe('station');
    expect(findAllTags(Reading.fields[0]?.tags ?? [], 'alias')).toHaveLength(2);
  });

  it('should retain the tags of types, variants and members', () => {
    expect(Position.tags).toEqual([{ name: 'kind', value: Kind.Plain }]);
    expect(Shape.tags).toEqual([{ name: 'kind', value: Kind.Sum }]);
    expect(Unit.tags).toEqual([{ name: 'kind', value: Kind.Plain }]);
    expect(Kind.tags).toEqual([]);
    expect(Unit.members).toEqual([
      {
        name: 'Celsius',
        value: 1,
        tags: [
          { name: 'alias', value: 'celsius' },
          { name: 'marker', value: 'C' }
        ]
      },
      { name: 'Kelvin', value: 7, tags: [] }
    ]);
    expect(Shape.Point.tags).toEqual([{ name: 'alias', value: 'dot' }]);
    expect(findTag(Shape.Box.fields[0]?.tags ?? [], 'recipient')).toEqual({ name: 'recipient' });
    expect(Shape.Circle.fields).toEqual([{ name: 'value', property: 'value', tags: [] }]);
  });

  it("should read a message's fields by their properties", () => {
    const reading = new Reading({
      value: 1.5,
      station: 3,
      labels: ['north'],
      position: new Position({ across: 1, down: 2 }),
      shape: new Shape.Box({ width: 4, height: 5 }),
      unit: Unit.Kelvin
    });
    const values = Reading.fields.map((field) => (reading as unknown as Record<string, unknown>)[field.property]);

    expect(values.slice(0, 3)).toEqual([1.5, 3, ['north']]);
  });
});
