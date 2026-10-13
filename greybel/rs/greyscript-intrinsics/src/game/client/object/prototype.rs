use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use greybel_interpreter::value::{CLASS_ID, Key};
use greybel_interpreter::{Heap, Map, Pin, Value, Vm};
use greybel_symbols::names::ISA;

use super::{GameObject, Kind, ObjectKey};

/// Per-VM type maps, GC-pinned to outlive their objects, and initial object fields.
struct TypeTable {
    maps: [Map; Kind::ALL.len()],
    fields: Rc<FieldKeyset>,
    _pins: Vec<Pin>,
}

/// Per-VM `__isa` and `classID` keys, class IDs, and allocated object maps.
struct FieldKeyset {
    isa: Key,
    class_id: Key,
    class_ids: [Value; Kind::ALL.len()],
    /// Most recently allocated map for each kind and object.
    objects: RefCell<HashMap<(Kind, ObjectKey), Map>>,
}

/// Kind type map and shared [`FieldKeyset`].
/// Construction requires only the heap, allowing use in async-native finish steps.
pub(in crate::game) struct Prototype {
    kind: Kind,
    map: Map,
    fields: Rc<FieldKeyset>,
}

impl Prototype {
    /// Returns the object map for `object`, reusing a live map for an equal server object.
    /// Grey Hack reissues equal objects (`HelperScript.AddShell` and similar APIs), and script objects use identity equality, so `get_shell == get_shell`.
    pub(in crate::game) fn instantiate(&self, heap: &mut Heap, object: GameObject) -> Value {
        let fields = &self.fields;
        let key = (self.kind, object.key());
        let current = fields.objects.borrow().get(&key).copied();

        if let Some(map) = current.filter(|&map| self.is_instance(heap, map, &key.1)) {
            return Value::Map(map);
        }

        let entries = [
            (&fields.isa, Value::Map(self.map)),
            (
                &fields.class_id,
                fields.class_ids[self.kind as usize].clone(),
            ),
        ];
        let map = heap.add_host_object(entries, object);
        let mut objects = fields.objects.borrow_mut();

        objects.retain(|_, &mut map| map.is_live(heap));
        objects.insert(key, map);

        Value::Map(map)
    }

    /// Tests whether `map` remains this kind's live object for `key`, rather than a GC-freed map.
    fn is_instance(&self, heap: &Heap, map: Map, key: &ObjectKey) -> bool {
        map.is_live(heap)
            && matches!(map.get_key(heap, &self.fields.isa), Some(Value::Map(isa)) if *isa == self.map)
            && map
                .handle::<GameObject>(heap)
                .is_some_and(|object| object.key() == *key)
    }
}

/// Tests whether `map` is a class type map allocated by this VM.
pub(super) fn is_type_map(vm: &Vm, map: Map) -> bool {
    vm.host_data::<TypeTable>()
        .is_some_and(|types| types.maps.contains(&map))
}

/// Returns this VM's prototype for `kind`, constructing type maps on first use.
pub(in crate::game) fn prototype(vm: &mut Vm, kind: Kind) -> Prototype {
    if let Some(types) = vm.host_data::<TypeTable>() {
        return Prototype {
            kind,
            map: types.maps[kind as usize],
            fields: Rc::clone(&types.fields),
        };
    }

    let mut pins = Vec::new();
    let maps = Kind::ALL.map(|kind| {
        let entries: Vec<(Value, Value)> = kind
            .methods()
            .iter()
            .map(|&(name, native)| (Value::from(name), vm.native_function(native)))
            .collect();
        let map = vm.heap_mut().add_sealed_map(entries);

        pins.push(vm.pin(Value::Map(map)));

        map
    });
    let fields = Rc::new(FieldKeyset {
        isa: Key::new(ISA),
        class_id: Key::new(CLASS_ID),
        class_ids: Kind::ALL.map(|kind| Value::from(kind.class_id())),
        objects: RefCell::default(),
    });

    vm.host_data_or_insert_with(|| TypeTable {
        maps,
        fields: Rc::clone(&fields),
        _pins: pins,
    });

    Prototype {
        kind,
        map: maps[kind as usize],
        fields,
    }
}
