import { Requirement, Violation } from '../definition.generated.js';
import { ChannelError } from '../errors.js';
import type { Pattern } from './pattern.js';

export class FieldValidation {
  static enterField(path: number[] | undefined, position: number): number[] {
    const fieldPath = path ?? [];

    fieldPath.push(position);

    return fieldPath;
  }

  static advance(path: number[] | undefined): void {
    if (path === undefined) return;
    path[path.length - 1]!++;
  }

  static invalidAt(path: number[] | undefined, requirement: Requirement): ChannelError<never> {
    return ChannelError.invalid(new Violation({ path: path === undefined ? [] : [...path], requirement }));
  }

  static validateLength(path: number[] | undefined, length: number, min: number, max: number): void {
    if (length < min) throw FieldValidation.invalidAt(path, Requirement.MinLength);
    if (length > max) throw FieldValidation.invalidAt(path, Requirement.MaxLength);
  }

  static validateRange<T extends number | bigint>(
    path: number[] | undefined,
    value: T,
    min: number | bigint,
    max: number | bigint
  ): T {
    if (!(value >= min)) throw FieldValidation.invalidAt(path, Requirement.Minimum);
    if (value > max) throw FieldValidation.invalidAt(path, Requirement.Maximum);

    return value;
  }

  static validatePattern(
    path: number[] | undefined,
    pattern: Pattern | undefined,
    bytes: Uint8Array,
    start: number,
    end: number
  ): void {
    if (pattern !== undefined && !pattern.matches(bytes, start, end))
      throw FieldValidation.invalidAt(path, Requirement.Pattern);
  }
}
