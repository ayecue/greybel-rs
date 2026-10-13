
export function slotCount(consumer: object): number {
  const items: unknown = Reflect.get(consumer, "_items");
  return Array.isArray(items) ? items.length : 0;
}
