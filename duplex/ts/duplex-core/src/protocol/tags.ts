export type TagValue = boolean | number | bigint | string;

export interface Tag {
  readonly name: string;
  readonly value?: TagValue;
}

export interface FieldTagContainer {
  readonly name: string;
  readonly property: string;
  readonly tags: readonly Tag[];
}

export function findTag(tags: readonly Tag[], name: string): Tag | undefined {
  return tags.find((tag) => tag.name === name);
}

export function findAllTags(tags: readonly Tag[], name: string): Tag[] {
  return tags.filter((tag) => tag.name === name);
}

export function fieldWithTag(fields: readonly FieldTagContainer[], tag: string): FieldTagContainer | undefined {
  return fields.find((field) => findTag(field.tags, tag) !== undefined);
}

export interface MemberTagContainer {
  readonly name: string;
  readonly value: number;
  readonly tags: readonly Tag[];
}
