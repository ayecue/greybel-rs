use std::rc::Rc;

use super::{InboundBatch, InboundBufferManager, OutboundBatch};
use crate::binary::{CONTINUATION, PAYLOAD, Varuint};
use crate::error::{DecodeFailure, Error};

const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_LENGTH_BYTES: usize = 4;
const FIRST_READ_BYTES: usize = 4 * 1024;
const MOST_READ_BYTES: usize = 64 * 1024;
const SHIFT: usize = 7;

/// Decodes varuint-length-prefixed batches from a byte stream.
///
/// Reads grow from 4 KiB to 64 KiB. Resulting batches share read buffers.
pub struct BatchStream {
    buffer: Rc<[u8]>,
    start: usize,
    end: usize,
    read_size: usize,
    read_max_size: usize,
    buffers: InboundBufferManager,
}

impl Default for BatchStream {
    fn default() -> Self {
        Self {
            buffer: Rc::from([]),
            start: 0,
            end: 0,
            read_size: FIRST_READ_BYTES,
            read_max_size: 0,
            buffers: InboundBufferManager::default(),
        }
    }
}

impl std::fmt::Debug for BatchStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BatchStream")
            .field("unread", &(self.end - self.start))
            .finish_non_exhaustive()
    }
}

impl BatchStream {
    /// Maximum encoded batch-length bytes.
    pub const RESERVED_PREFIX_BYTES: usize = MAX_LENGTH_BYTES;

    /// Prepends the varuint length of `batch` for stream transmission.
    pub fn frame(batch: &mut OutboundBatch) {
        batch.prepend(Varuint::new(batch.len() as u64).as_bytes());
    }

    /// Returns the buffer tail for the next stream read.
    ///
    /// Pass the written byte count to [`read`](Self::read).
    pub fn take_remains(&mut self) -> &mut [u8] {
        let size = self.read_size.max(self.pending_size());
        let fits = self.start + size <= self.buffer.len() && self.end < self.buffer.len();

        if !fits || Rc::get_mut(&mut self.buffer).is_none() {
            self.relocate(size);
        }

        let end = self.end;
        let read_target = &mut Rc::get_mut(&mut self.buffer)
            .expect("the stream reads into a buffer of its own")[end..];

        self.read_max_size = read_target.len();

        read_target
    }

    /// Consumes `count` newly read tail bytes and calls `receive` for each complete batch.
    ///
    /// # Errors
    ///
    /// Returns an error for batch lengths over 64 MiB or length prefixes over four bytes, then resets the stream.
    pub fn read(
        &mut self,
        count: usize,
        mut receive: impl FnMut(InboundBatch),
    ) -> Result<(), Error> {
        self.end += count;
        if count >= self.read_max_size && self.read_size < MOST_READ_BYTES {
            self.read_size *= 2;
        }

        loop {
            let length = match Self::length(&self.buffer[self.start..self.end]) {
                Ok(Some(length)) => length,
                Ok(None) => return Ok(()),
                Err(error) => {
                    self.start = 0;
                    self.end = 0;

                    return Err(error);
                }
            };
            let (length, header) = length;
            let begin = self.start + header;

            if begin + length > self.end {
                return Ok(());
            }

            receive(InboundBatch::new(
                Rc::clone(&self.buffer),
                begin..begin + length,
            ));
            self.start = begin + length;
        }
    }

    /// Processes `bytes`, calls `receive` for complete batches, and retains an incomplete trailing batch.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`BatchStream::read`].
    pub fn process(
        &mut self,
        mut bytes: &[u8],
        mut receive: impl FnMut(InboundBatch),
    ) -> Result<(), Error> {
        while !bytes.is_empty() {
            let read_target = self.take_remains();
            let count = read_target.len().min(bytes.len());

            read_target[..count].copy_from_slice(&bytes[..count]);
            bytes = &bytes[count..];
            self.read(count, &mut receive)?;
        }

        Ok(())
    }

    /// Splits `batch` into length-prefixed batches and calls `receive` for each.
    ///
    /// # Errors
    ///
    /// Returns [`BatchStream::read`] errors, or an error for an incomplete trailing length or payload in `batch`.
    pub fn split(batch: &InboundBatch, mut receive: impl FnMut(InboundBatch)) -> Result<(), Error> {
        let range = batch.range();
        let bytes = batch.as_slice();
        let mut at = 0;

        while at < bytes.len() {
            let Some((length, header)) = Self::length(&bytes[at..])? else {
                return Err(Self::cut_short());
            };
            let begin = at + header;

            if begin + length > bytes.len() {
                return Err(Self::cut_short());
            }

            let start = range.start + begin;

            receive(InboundBatch::new(
                Rc::clone(batch.buffer()),
                start..start + length,
            ));
            at = begin + length;
        }

        Ok(())
    }

    fn cut_short() -> Error {
        Error::Decode(DecodeFailure::BatchEnd)
    }

    /// Returns total encoded size of the current batch, or `0` for an incomplete or invalid length.
    fn pending_size(&self) -> usize {
        match Self::length(&self.buffer[self.start..self.end]) {
            Ok(Some((length, header))) => header + length,
            _ => 0,
        }
    }

    /// Moves pending bytes to the start of an unshared buffer with at least `size` bytes.
    ///
    /// Reuses the current buffer when possible, then a retained buffer, or allocates.
    fn relocate(&mut self, size: usize) {
        let unread = self.end - self.start;
        let size = size.max(unread + 1);

        if self.buffer.len() >= size
            && let Some(buffer) = Rc::get_mut(&mut self.buffer)
        {
            buffer.copy_within(self.start..self.end, 0);
        } else {
            let mut fresh = self.buffers.recycle_by_size(size);

            Rc::get_mut(&mut fresh).expect("a buffer nothing else references")[..unread]
                .copy_from_slice(&self.buffer[self.start..self.end]);

            let old = std::mem::replace(&mut self.buffer, fresh);

            self.buffers.retain(old);
        }

        self.start = 0;
        self.end = unread;
    }

    fn length(bytes: &[u8]) -> Result<Option<(usize, usize)>, Error> {
        let mut length = 0;

        for (index, &byte) in bytes.iter().take(MAX_LENGTH_BYTES).enumerate() {
            length |= usize::from(byte & PAYLOAD) << (SHIFT * index);
            if byte & CONTINUATION == 0 {
                if length > MAX_BATCH_BYTES {
                    return Err(Error::Decode(DecodeFailure::BatchSize {
                        length,
                        maximum: MAX_BATCH_BYTES,
                    }));
                }

                return Ok(Some((length, index + 1)));
            }
        }

        if bytes.len() >= MAX_LENGTH_BYTES {
            return Err(Error::Decode(DecodeFailure::BatchLength {
                maximum: MAX_LENGTH_BYTES,
            }));
        }

        Ok(None)
    }
}
