use std::collections::VecDeque;

use duplex_core::Varuint;

const COMPACT_BYTES: usize = 64 * 1024;

/// Unacknowledged batch records for long-poll and streaming responses. Sequence numbers start at 1.
pub(crate) struct Backlog {
    records: Vec<u8>,
    /// Stores record start and payload offsets in `records`; the first queued record has sequence `first`.
    record_queue: VecDeque<Record>,
    first: u64,
    limit: usize,
    overloaded: bool,
}

#[derive(Clone, Copy)]
struct Record {
    start: usize,
    batch_start: usize,
}

impl Backlog {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            records: Vec::new(),
            record_queue: VecDeque::new(),
            first: 1,
            limit,
            overloaded: false,
        }
    }

    /// Appends `batch` until acknowledged. If the byte limit would be exceeded, marks the transport overloaded and
    /// returns `false`.
    pub(crate) fn push(&mut self, batch: &[u8]) -> bool {
        let sequence = self.last() + 1;
        let length = Varuint::size(sequence) + batch.len();
        let start = self.records.len();
        let footprint = start - self.head() + Varuint::size(length as u64) + length;

        if self.overloaded || footprint > self.limit {
            self.overloaded = true;

            return false;
        }

        self.records
            .extend_from_slice(Varuint::new(length as u64).as_bytes());
        self.records
            .extend_from_slice(Varuint::new(sequence).as_bytes());
        self.record_queue.push_back(Record {
            start,
            batch_start: self.records.len(),
        });
        self.records.extend_from_slice(batch);

        true
    }

    /// Drops batches acknowledged through `sequence` and compacts storage when beneficial.
    pub(crate) fn acknowledge(&mut self, sequence: u64) {
        while self.first <= sequence && self.record_queue.pop_front().is_some() {
            self.first += 1;
        }

        if self.record_queue.is_empty() {
            self.records.clear();
        } else if self.head() >= COMPACT_BYTES && self.head() * 2 >= self.records.len() {
            let trim_len = self.head();

            self.records.drain(..trim_len);
            for record in &mut self.record_queue {
                record.start -= trim_len;
                record.batch_start -= trim_len;
            }
        }
    }

    /// Returns the latest queued sequence, or 0 before any batch is queued.
    pub(crate) fn last(&self) -> u64 {
        self.first + self.record_queue.len() as u64 - 1
    }

    pub(crate) fn has_after(&self, sequence: u64) -> bool {
        self.last() > sequence
    }

    pub(crate) fn is_overloaded(&self) -> bool {
        self.overloaded
    }

    /// Returns records after `sequence` and the latest sequence, or `None` without newer batches.
    pub(crate) fn records_after(&self, sequence: u64) -> Option<(&[u8], u64)> {
        let index = self.index_after(sequence)?;

        Some((&self.records[self.record_queue[index].start..], self.last()))
    }

    /// Calls `each` for batches after `sequence` and returns the latest sequence, or `None` without newer batches.
    pub(crate) fn each_after(
        &self,
        sequence: u64,
        mut each: impl FnMut(u64, &[u8]),
    ) -> Option<u64> {
        let index = self.index_after(sequence)?;

        for (offset, record) in self.record_queue.iter().enumerate().skip(index) {
            let end = self
                .record_queue
                .get(offset + 1)
                .map_or(self.records.len(), |next| next.start);

            each(
                self.first + offset as u64,
                &self.records[record.batch_start..end],
            );
        }

        Some(self.last())
    }

    fn index_after(&self, sequence: u64) -> Option<usize> {
        if !self.has_after(sequence) {
            return None;
        }

        Some(usize::try_from(sequence.saturating_sub(self.first - 1)).unwrap_or(usize::MAX))
    }

    fn head(&self) -> usize {
        self.record_queue
            .front()
            .map_or(self.records.len(), |record| record.start)
    }
}

#[cfg(test)]
#[path = "tests/backlog.rs"]
mod tests;
