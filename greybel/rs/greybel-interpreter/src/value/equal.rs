use rustc_hash::FxHashSet;

use super::{Heap, List, Map, Value};

#[derive(Default)]
struct VisitSet {
    lists: FxHashSet<(List, List)>,
    maps: FxHashSet<(Map, Map)>,
}

impl Heap {
    #[must_use]
    pub fn equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::List(_), Value::List(_)) | (Value::Map(_), Value::Map(_)) => {
                self.equal_contents(a, b, &mut VisitSet::default())
            }
            _ => self.equal_shallow(a, b),
        }
    }

    fn equal_shallow(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Number(x), Value::Number(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            (Value::Function(x), Value::Function(y)) => {
                x.closure(self).body == y.closure(self).body
            }
            _ => false,
        }
    }

    fn equal_contents(&self, a: &Value, b: &Value, visit_set: &mut VisitSet) -> bool {
        match (a, b) {
            (Value::List(x), Value::List(y)) => {
                if x == y || !visit_set.lists.insert((*x, *y)) {
                    return true;
                }

                let (x, y) = (x.items(self), y.items(self));

                x.len() == y.len()
                    && x.iter()
                        .zip(y)
                        .all(|(a, b)| self.equal_contents(a, b, visit_set))
            }
            (Value::Map(x), Value::Map(y)) => {
                if x == y {
                    return true;
                }

                if x.is_host_object(self) || y.is_host_object(self) {
                    return false;
                }

                if !visit_set.maps.insert((*x, *y)) {
                    return true;
                }

                let (left, right) = (*x, *y);

                left.entries(self).len() == right.entries(self).len()
                    && left.entries(self).iter().all(|(key, value)| {
                        right
                            .get(self, key)
                            .is_some_and(|other| self.equal_contents(value, other, visit_set))
                    })
            }
            _ => self.equal_shallow(a, b),
        }
    }
}
