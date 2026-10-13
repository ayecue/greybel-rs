use std::rc::Rc;

use super::InboundBatch;

const SMALLEST_BUFFER_BYTES: usize = 4 * 1024;
const MAX_STORED_BUFFER_BYTES: usize = 64 * 1024;
const MAX_STORED_BUFFERS: usize = 2;

/// Reuses unreferenced inbound batch buffers, retaining at most two buffers no larger than 64 KiB each.
#[derive(Default)]
pub struct InboundBufferManager {
    buffer_storage: Vec<Rc<[u8]>>,
}

impl std::fmt::Debug for InboundBufferManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InboundBufferManager")
            .field("retained", &self.buffer_storage.len())
            .finish()
    }
}

impl InboundBufferManager {
    /// Creates a `length`-byte batch initialized by `fill`.
    pub fn batch(&mut self, length: usize, fill: impl FnOnce(&mut [u8])) -> InboundBatch {
        let mut buffer = self.recycle_by_size(length);

        fill(&mut Rc::get_mut(&mut buffer).expect("a buffer nothing else references")[..length]);

        let batch = InboundBatch::new(Rc::clone(&buffer), 0..length);

        self.retain(buffer);

        batch
    }

    /// Creates a batch by copying `bytes`.
    pub fn copy(&mut self, bytes: &[u8]) -> InboundBatch {
        self.batch(bytes.len(), |batch| batch.copy_from_slice(bytes))
    }

    /// Returns an unshared buffer of at least `size` bytes, reusing retained capacity when sufficient.
    ///
    /// May discard an unused retained buffer. Allocations up to 64 KiB use at least 4 KiB and round `size` up to a
    /// power of two; larger allocations use `size`.
    pub(crate) fn recycle_by_size(&mut self, size: usize) -> Rc<[u8]> {
        if size > MAX_STORED_BUFFER_BYTES {
            return Self::allocate_buffer(size);
        }

        if let Some(index) = self
            .buffer_storage
            .iter_mut()
            .position(|buffer| buffer.len() >= size && Rc::get_mut(buffer).is_some())
        {
            return self.buffer_storage.swap_remove(index);
        }

        if let Some(index) = self
            .buffer_storage
            .iter_mut()
            .position(|buffer| Rc::get_mut(buffer).is_some())
        {
            self.buffer_storage.swap_remove(index);
        }

        Self::allocate_buffer(size.next_power_of_two().max(SMALLEST_BUFFER_BYTES))
    }

    fn allocate_buffer(size: usize) -> Rc<[u8]> {
        std::iter::repeat_n(0, size).collect()
    }

    /// Retains `buffer` for reuse once unshared.
    ///
    /// Discards buffers over 64 KiB and stores at most two buffers.
    pub(crate) fn retain(&mut self, buffer: Rc<[u8]>) {
        if buffer.len() <= MAX_STORED_BUFFER_BYTES && self.buffer_storage.len() < MAX_STORED_BUFFERS
        {
            self.buffer_storage.push(buffer);
        }
    }
}
