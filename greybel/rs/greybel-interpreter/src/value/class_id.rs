use super::{Heap, Key, Map};

pub const CLASS_ID: &str = "classID";

thread_local! {
    static CLASS_ID_KEY: Key = Key::new(CLASS_ID);
}

impl Map {
    #[must_use]
    pub fn class_id(self, heap: &Heap) -> Option<String> {
        CLASS_ID_KEY.with(|key| {
            self.get_key(heap, key)
                .map(|value| heap.display(value).to_string())
        })
    }
}
