import type { BinaryReader } from '../../binary/reader.js';
import { FieldValidation } from '../../binary/validations.js';
import { Requirement } from '../../definition.generated.js';

export class Validation {
  static prepareBody(validating: boolean, limit: number, body: BinaryReader): void {
    if (!validating) return;
    if (body.end - body.offset > limit) throw FieldValidation.invalidAt(undefined, Requirement.Limit);
    body.validating = true;
  }

  static validateItem(validating: boolean, items: readonly [number, number], count: number): void {
    if (validating && count > items[1]) throw FieldValidation.invalidAt(undefined, Requirement.MaxLength);
  }

  static validateEnd(validating: boolean, items: readonly [number, number], count: number): void {
    if (validating && count < items[0]) throw FieldValidation.invalidAt(undefined, Requirement.MinLength);
  }
}
