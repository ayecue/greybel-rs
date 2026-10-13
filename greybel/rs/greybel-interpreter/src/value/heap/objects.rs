use super::AllocationSize;

const MARK_FLAG: u8 = 1;
const SEAL_FLAG: u8 = 2;

#[derive(Debug)]
pub(super) struct ObjectTable<T> {
    items: Vec<Option<T>>,
    vacant_slots: Vec<u32>,
    flags: Vec<u8>,
}

impl<T> Default for ObjectTable<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            vacant_slots: Vec::new(),
            flags: Vec::new(),
        }
    }
}

impl<T> ObjectTable<T> {
    pub(super) fn insert(&mut self, item: T) -> u32 {
        if let Some(index) = self.vacant_slots.pop() {
            self.items[index as usize] = Some(item);
            self.flags[index as usize] = 0;

            return index;
        }

        self.items.push(Some(item));
        self.flags.push(0);

        u32::try_from(self.items.len() - 1).expect("fewer than 2^32 heap objects")
    }

    pub(super) fn get(&self, index: u32) -> &T {
        self.items[index as usize]
            .as_ref()
            .expect("a live heap object")
    }

    pub(super) fn get_mut(&mut self, index: u32) -> &mut T {
        self.items[index as usize]
            .as_mut()
            .expect("a live heap object")
    }

    pub(super) fn contains(&self, index: u32) -> bool {
        self.items.get(index as usize).is_some_and(Option::is_some)
    }

    pub(super) fn len(&self) -> usize {
        self.items.len() - self.vacant_slots.len()
    }

    pub(super) fn is_sealed(&self, index: u32) -> bool {
        self.flags[index as usize] & SEAL_FLAG != 0
    }

    pub(super) fn seal(&mut self, index: u32) {
        self.flags[index as usize] |= SEAL_FLAG;
    }

    pub(super) fn unmark_all(&mut self) {
        for flags in &mut self.flags {
            *flags &= !MARK_FLAG;
        }
    }

    pub(super) fn mark(&mut self, index: u32) -> bool {
        let flags = &mut self.flags[index as usize];
        let first = *flags & MARK_FLAG == 0;

        *flags |= MARK_FLAG;

        first
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct SweepResult {
    pub released_objects: usize,
    pub current_allocation_size: usize,
}

impl std::ops::Add for SweepResult {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            released_objects: self.released_objects + other.released_objects,
            current_allocation_size: self.current_allocation_size + other.current_allocation_size,
        }
    }
}

impl<T: AllocationSize> ObjectTable<T> {
    pub(super) fn sweep(&mut self, mut recycle: impl FnMut(T)) -> SweepResult {
        let mut result = SweepResult::default();

        for (index, item) in self.items.iter_mut().enumerate() {
            if self.flags[index] & MARK_FLAG != 0 {
                result.current_allocation_size +=
                    item.as_ref().map_or(0, AllocationSize::allocation_size);
            } else if let Some(old) = item.take() {
                recycle(old);
                self.vacant_slots
                    .push(u32::try_from(index).expect("fewer than 2^32 heap objects"));
                result.released_objects += 1;
            }
        }

        result
    }
}
