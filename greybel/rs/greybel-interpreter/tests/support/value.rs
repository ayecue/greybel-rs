use greybel_interpreter::{Heap, Map, Value};

pub fn map(heap: &mut Heap, entries: Vec<(Value, Value)>) -> Value {
    Value::Map(heap.add_map_from(entries))
}

pub fn list(heap: &mut Heap, items: Vec<Value>) -> Value {
    Value::List(heap.add_list(items))
}

/// Heap fixture with equal and unequal lists and maps; `table` maps list `a` to `"test"`.
pub struct Equality {
    pub heap: Heap,
    pub a: Value,
    pub b: Value,
    pub c: Value,
    pub table: Map,
    pub x: Value,
    pub y: Value,
}

impl Default for Equality {
    fn default() -> Self {
        let mut heap = Heap::default();
        let a = list(&mut heap, vec![0.0.into(), 0.0.into(), 1.0.into()]);
        let b = list(&mut heap, vec![0.0.into(), 0.0.into(), 1.0.into()]);
        let c = list(&mut heap, vec![0.0.into(), 1.0.into(), 0.0.into()]);
        let table = heap.add_map_from(vec![(a.clone(), "test".into())]);
        let x = map(
            &mut heap,
            vec![("was".into(), 1.0.into()), ("second".into(), 2.0.into())],
        );
        let y = map(
            &mut heap,
            vec![("second".into(), 2.0.into()), ("was".into(), 1.0.into())],
        );

        Self {
            heap,
            a,
            b,
            c,
            table,
            x,
            y,
        }
    }
}

/// Creates two maps that each contain a self-reference under `"self"`.
pub fn cyclic_maps() -> (Heap, Map, Map) {
    let mut heap = Heap::default();
    let a = heap.add_map();
    let b = heap.add_map();

    a.set(&mut heap, "self".into(), Value::Map(a));
    b.set(&mut heap, "self".into(), Value::Map(b));

    (heap, a, b)
}

/// Returns rendered keys of `map` in iteration order.
pub fn keys(heap: &Heap, map: Map) -> Vec<String> {
    map.entries(heap)
        .keys()
        .map(|key| heap.display(key).to_string())
        .collect()
}

/// Creates a map with `"a"`, `"b"`, and `"c"` each set to 1.
pub fn abc() -> (Heap, Map) {
    let mut heap = Heap::default();
    let map = heap.add_map_from(["a", "b", "c"].map(|key| (key.into(), 1.0.into())));

    (heap, map)
}
