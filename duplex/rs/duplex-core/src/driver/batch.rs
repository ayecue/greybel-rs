use std::cell::RefCell;
use std::fmt;
use std::ops::Range;
use std::rc::{Rc, Weak};

const RECYCLE_BUFFER_SIZE: usize = 16;

/// Outbound batch with prefix space reserved for driver framing.
///
/// Drop returns the buffer to its transport pool.
pub struct OutboundBatch {
    bytes: Vec<u8>,
    start: usize,
    pool: Weak<BatchPool>,
}

impl OutboundBatch {
    pub(crate) fn new(bytes: Vec<u8>, start: usize, pool: &Rc<BatchPool>) -> Self {
        Self {
            bytes,
            start,
            pool: Rc::downgrade(pool),
        }
    }

    #[must_use]
    /// Returns batch bytes without the unused prefix.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[self.start..]
    }

    #[must_use]
    /// Returns the batch byte length.
    pub fn len(&self) -> usize {
        self.bytes.len() - self.start
    }

    #[must_use]
    /// Returns `true` for an empty batch.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Adds `prefix` before the existing batch bytes.
    ///
    /// Uses reserved space when sufficient; otherwise shifts existing bytes.
    pub fn prepend(&mut self, prefix: &[u8]) {
        if prefix.len() <= self.start {
            self.start -= prefix.len();
            self.bytes[self.start..self.start + prefix.len()].copy_from_slice(prefix);
        } else {
            self.bytes.splice(..self.start, prefix.iter().copied());
            self.start = 0;
        }
    }

    #[must_use]
    /// Consumes the batch and returns a `Vec<u8>` without the unused reserved prefix.
    ///
    /// Does not return the buffer to the transport pool.
    pub fn into_vec(mut self) -> Vec<u8> {
        let mut bytes = std::mem::take(&mut self.bytes);

        bytes.drain(..self.start);

        bytes
    }
}

impl From<Vec<u8>> for OutboundBatch {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            start: 0,
            pool: Weak::new(),
        }
    }
}

impl From<&[u8]> for OutboundBatch {
    fn from(bytes: &[u8]) -> Self {
        Self::from(bytes.to_vec())
    }
}

impl AsRef<[u8]> for OutboundBatch {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl fmt::Debug for OutboundBatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("OutboundBatch")
            .field(&self.as_slice())
            .finish()
    }
}

impl Drop for OutboundBatch {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            pool.release(std::mem::take(&mut self.bytes));
        }
    }
}

/// Inbound batch referencing a shared buffer range.
///
/// Clones share the buffer without copying bytes.
#[derive(Clone)]
pub struct InboundBatch {
    buffer: Rc<[u8]>,
    range: Range<usize>,
}

impl InboundBatch {
    #[must_use]
    /// Creates a batch referencing `buffer[range]`.
    ///
    /// # Panics
    ///
    /// Panics when `range.start > range.end` or `range.end > buffer.len()`.
    pub fn new(buffer: Rc<[u8]>, range: Range<usize>) -> Self {
        assert!(
            range.start <= range.end && range.end <= buffer.len(),
            "a batch lies within its buffer"
        );

        Self { buffer, range }
    }

    #[must_use]
    /// Returns the referenced bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.range.clone()]
    }

    pub(crate) fn buffer(&self) -> &Rc<[u8]> {
        &self.buffer
    }

    pub(crate) fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

impl<T: Into<Rc<[u8]>>> From<T> for InboundBatch {
    fn from(bytes: T) -> Self {
        let buffer = bytes.into();
        let range = 0..buffer.len();

        Self { buffer, range }
    }
}

impl AsRef<[u8]> for InboundBatch {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl fmt::Debug for InboundBatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("InboundBatch")
            .field(&self.as_slice())
            .finish()
    }
}

/// Pools outbound batch buffers for reuse.
#[derive(Default)]
pub(crate) struct BatchPool(RefCell<Vec<Vec<u8>>>);

impl BatchPool {
    /// Reuses a retained buffer or allocates at least `capacity` bytes.
    pub(crate) fn take(&self, capacity: usize) -> Vec<u8> {
        self.0
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(capacity))
    }

    pub(crate) fn release(&self, mut bytes: Vec<u8>) {
        let mut buffer_storage = self.0.borrow_mut();

        if buffer_storage.len() < RECYCLE_BUFFER_SIZE && bytes.capacity() > 0 {
            bytes.clear();
            buffer_storage.push(bytes);
        }
    }
}

#[cfg(test)]
#[path = "tests/batch.rs"]
mod tests;
