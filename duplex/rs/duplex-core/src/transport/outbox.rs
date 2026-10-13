use std::collections::VecDeque;
use std::collections::hash_map::Entry;

use rustc_hash::FxHashMap;

use super::aliases::LocalAliasManager;
use crate::binary::{BinaryReader, BinaryWriter};
use crate::channel::Bundle;
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Role};
use crate::error::Error;
use crate::frame::{BatchWriter, CORE_MAJOR, FrameEncoder, LAST_FRAME, Route};

const MAX_BATCH_BYTES: usize = 64 * 1024;
const MAX_BUFFERED_BYTES: usize = 16 * 1024 * 1024;
const MIN_FRAGMENT_BYTES: usize = 16 * 1024;
/// Reserves bytes before outbox frames for a batch header and slice key so a single-slice batch can reuse the
/// outbox buffer.
const RESERVED_HEADER_BYTES: usize = 16;

struct Run {
    connection: Option<ConnectionId>,
    bundle: Bundle,
    end: usize,
    last: usize,
}

struct Outbox {
    from: Role,
    channel: u32,
    /// Empty-outbox frame offset after the driver prefix, batch header, and slice key.
    head: usize,
    frames: BinaryWriter,
    unsent_start: usize,
    next_piece_start: Option<usize>,
    runs: VecDeque<Run>,
    ready: bool,
}

impl Outbox {
    fn new(from: Role, channel: u32, reserved_prefix: usize) -> Self {
        let head = reserved_prefix + RESERVED_HEADER_BYTES;
        let mut frames = BinaryWriter::new();

        frames.pad(head);

        Self {
            from,
            channel,
            head,
            frames,
            unsent_start: head,
            next_piece_start: None,
            runs: VecDeque::new(),
            ready: false,
        }
    }

    fn route(&self, connection: Option<ConnectionId>) -> Route {
        Route {
            from: self.from,
            channel: self.channel,
            connection,
        }
    }

    fn append(
        &mut self,
        connection: Option<ConnectionId>,
        bundle: Bundle,
        end: usize,
        last: usize,
    ) {
        match self.runs.back_mut() {
            Some(run) if run.connection == connection && run.bundle == bundle => {
                run.end = end;
                run.last = last;
            }
            _ => self.runs.push_back(Run {
                connection,
                bundle,
                end,
                last,
            }),
        }
    }

    fn plan(&self, bundle: Bundle, share: usize, slices: &mut Vec<PlannedSlice>, outbox: usize) {
        let mut planner = Planner {
            frames: self.frames.as_slice(),
            outbox,
            share_left: share,
            piece_size: share.max(MIN_FRAGMENT_BYTES),
            has_frames: false,
            slices,
        };
        let mut runs = self.runs.iter().take_while(|run| run.bundle == bundle);

        if let Some(piece_start) = self.next_piece_start {
            if let Some(run) = runs.next() {
                planner.next_piece(run, self.unsent_start, piece_start);
            }

            return;
        }

        let mut start = self.unsent_start;

        for run in runs {
            if !planner.run(run, start) {
                return;
            }

            start = run.end;
        }
    }

    fn frame_end(frames: &[u8], at: usize, end: usize) -> Option<usize> {
        let mut reader = BinaryReader::new(&frames[at + 1..end]);
        let length = reader.length().ok()?;

        Some(at + 1 + reader.position() + length)
    }

    fn commit_fragment(&mut self, to: usize, ends: bool) {
        if ends {
            self.next_piece_start = None;
            self.take(to);
        } else {
            self.next_piece_start = Some(to);
        }
    }

    fn take(&mut self, to: usize) {
        self.unsent_start = to;
        while self.runs.front().is_some_and(|run| run.end <= to) {
            self.runs.pop_front();
        }

        if self.runs.is_empty() {
            self.frames.clear();
            self.frames.pad(self.head);
            self.unsent_start = self.head;
        } else if self.unsent_start >= MAX_BATCH_BYTES && self.unsent_start * 2 >= self.frames.len()
        {
            let trim_len = self.unsent_start - self.head;

            self.frames.remove_front(trim_len);
            for run in &mut self.runs {
                run.end -= trim_len;
                run.last -= trim_len;
            }

            if let Some(piece_start) = &mut self.next_piece_start {
                *piece_start -= trim_len;
            }

            self.unsent_start = self.head;
        }
    }

    /// Returns whether `slice` covers all remaining frames so the outbox buffer can be sent directly.
    fn is_all(&self, slice: &PlannedSlice) -> bool {
        slice.start == self.unsent_start
            && slice.end == self.frames.len()
            && self.next_piece_start.is_none()
    }

    /// Returns the common run bundle, or `None` if no runs exist or bundles differ.
    fn single_bundle(&self) -> Option<Bundle> {
        let bundle = self.runs.front()?.bundle;

        self.runs
            .iter()
            .all(|run| run.bundle == bundle)
            .then_some(bundle)
    }
}

struct PlannedSlice {
    outbox: usize,
    connection: Option<ConnectionId>,
    start: usize,
    end: usize,
    piece: Piece,
}

#[derive(Clone, Copy)]
enum Piece {
    Frames { last: usize },
    Fragment { ends: bool },
}

impl PlannedSlice {
    fn frames(
        outbox: usize,
        connection: Option<ConnectionId>,
        start: usize,
        end: usize,
        last: usize,
    ) -> Self {
        Self {
            outbox,
            connection,
            start,
            end,
            piece: Piece::Frames { last },
        }
    }

    fn fragment(
        outbox: usize,
        connection: Option<ConnectionId>,
        start: usize,
        end: usize,
        ends: bool,
    ) -> Self {
        Self {
            outbox,
            connection,
            start,
            end,
            piece: Piece::Fragment { ends },
        }
    }
}

struct Planner<'a> {
    frames: &'a [u8],
    outbox: usize,
    share_left: usize,
    piece_size: usize,
    has_frames: bool,
    slices: &'a mut Vec<PlannedSlice>,
}

impl Planner<'_> {
    fn run(&mut self, run: &Run, start: usize) -> bool {
        if run.end - start <= self.share_left {
            debug_assert_eq!(
                Outbox::frame_end(self.frames, run.last, run.end),
                Some(run.end)
            );
            self.share_left -= run.end - start;
            self.push(run, start, run.end, run.last);

            return self.share_left > 0;
        }

        let mut end = start;
        let mut last = None;

        while end < run.end {
            let Some(frame_end) = Outbox::frame_end(self.frames, end, run.end) else {
                return false;
            };
            let size = frame_end - end;

            if size <= self.share_left {
                self.share_left -= size;
                last = Some(end);
                end = frame_end;
                continue;
            }

            let first = !self.has_frames && last.is_none();

            if size > self.piece_size && (first || self.share_left >= MIN_FRAGMENT_BYTES) {
                if let Some(last) = last {
                    self.push(run, start, end, last);
                }

                let piece = if first {
                    self.piece_size
                } else {
                    self.share_left
                };
                let fragment =
                    PlannedSlice::fragment(self.outbox, run.connection, end, end + piece, false);

                self.slices.push(fragment);

                return false;
            }

            if first && size <= self.piece_size {
                self.share_left = 0;
                last = Some(end);
                end = frame_end;
            }

            break;
        }

        if let Some(last) = last {
            self.push(run, start, end, last);
        }

        end == run.end && self.share_left > 0
    }

    fn next_piece(&mut self, run: &Run, frame_start: usize, piece_start: usize) {
        let Some(frame_end) = Outbox::frame_end(self.frames, frame_start, run.end) else {
            return;
        };
        let piece_end = (piece_start + self.piece_size).min(frame_end);
        let fragment = PlannedSlice::fragment(
            self.outbox,
            run.connection,
            piece_start,
            piece_end,
            piece_end == frame_end,
        );

        self.slices.push(fragment);
    }

    fn push(&mut self, run: &Run, start: usize, end: usize, last: usize) {
        let frames = PlannedSlice::frames(self.outbox, run.connection, start, end, last);

        self.slices.push(frames);
        self.has_frames = true;
    }
}

pub(super) enum NextBatch {
    Ready { start: usize },
    Failed,
    Done,
}

#[derive(Default)]
pub(crate) struct FlushState {
    single_outbox: Option<(usize, Bundle)>,
    bundles: Vec<Bundle>,
    order: FxHashMap<Bundle, usize>,
    fronts: FxHashMap<Bundle, Vec<usize>>,
    rank: Vec<usize>,
    next: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OutboxKey {
    role: Role,
    channel: u32,
}

#[derive(Default)]
pub(super) struct OutboxContainer {
    outboxes: Vec<Outbox>,
    index: FxHashMap<OutboxKey, usize>,
    last: Option<(OutboxKey, usize)>,
    ready: Vec<usize>,
    pending_bytes: usize,
    slices: Vec<PlannedSlice>,
    header: BinaryWriter,
    pub(super) aliases: LocalAliasManager,
    pub(super) capacity: usize,
}

impl OutboxContainer {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            ..Self::default()
        }
    }

    pub(super) fn write(
        &mut self,
        route: Route,
        bundle: Bundle,
        minor: u16,
        max_frame: usize,
        write: impl FnOnce(&mut FrameEncoder<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if self.pending_bytes >= MAX_BUFFERED_BYTES {
            return Err(Error::Rejection(ErrorCode::Overload));
        }

        let index = self.outbox(route.from, route.channel);
        let outbox = &mut self.outboxes[index];
        let start = outbox.frames.len();
        let mut frames = FrameEncoder::new(&mut outbox.frames, minor);
        let outcome = write(&mut frames);
        let last = frames.last();
        let end = outbox.frames.len();

        if end - start > max_frame {
            outbox.frames.truncate(start);

            return Err(Error::Rejection(ErrorCode::Overload));
        }

        match last.filter(|&last| last >= start && end > start) {
            Some(last) => {
                if !outbox.ready {
                    outbox.ready = true;
                    self.ready.push(index);
                }

                outbox.append(route.connection, bundle, end, last);
                self.pending_bytes += end - start;
            }
            None => outbox.frames.truncate(start),
        }

        outcome
    }

    pub(super) fn is_full(&self) -> bool {
        self.pending_bytes >= MAX_BATCH_BYTES
    }

    pub(super) fn is_empty(&self) -> bool {
        self.pending_bytes == 0
    }

    pub(super) fn begin_flush(&self, flush: &mut FlushState) {
        flush.next = 0;
        flush.bundles.clear();
        flush.single_outbox = None;
        if let [index] = self.ready[..]
            && let Some(bundle) = self.outboxes[index].single_bundle()
        {
            flush.single_outbox = Some((index, bundle));

            return;
        }

        flush.order.clear();
        flush.fronts.clear();
        flush.rank.clear();
        flush.rank.resize(self.outboxes.len(), usize::MAX);
        for (rank, &index) in self.ready.iter().enumerate() {
            flush.rank[index] = rank;

            let outbox = &self.outboxes[index];

            for run in &outbox.runs {
                if let Entry::Vacant(entry) = flush.order.entry(run.bundle) {
                    entry.insert(flush.bundles.len());
                    flush.bundles.push(run.bundle);
                }
            }

            if let Some(front) = outbox.runs.front() {
                flush.fronts.entry(front.bundle).or_default().push(index);
            }
        }
    }

    /// Writes the next batch to `batch`.
    pub(super) fn take_next(
        &mut self,
        flush: &mut FlushState,
        batch: &mut BinaryWriter,
    ) -> NextBatch {
        if let Some((index, bundle)) = flush.single_outbox.take() {
            return self.take(bundle, &[index], batch);
        }

        loop {
            let at = flush.next;
            let Some(&bundle) = flush.bundles.get(at) else {
                return NextBatch::Done;
            };

            flush.next += 1;

            let Some(mut sharers) = flush.fronts.remove(&bundle) else {
                continue;
            };

            sharers.sort_unstable_by_key(|&index| {
                flush.rank.get(index).copied().unwrap_or(usize::MAX)
            });

            let batch_start = self.take(bundle, &sharers, batch);

            for index in sharers {
                let Some(front) = self.outboxes[index].runs.front() else {
                    continue;
                };

                if flush
                    .order
                    .get(&front.bundle)
                    .is_some_and(|&order| order > at)
                {
                    flush.fronts.entry(front.bundle).or_default().push(index);
                }
            }

            return batch_start;
        }
    }

    pub(super) fn end_flush(&mut self) {
        let outboxes = &mut self.outboxes;

        self.ready.retain(|&index| {
            let outbox = &mut outboxes[index];

            outbox.ready = !outbox.runs.is_empty();

            outbox.ready
        });
    }

    fn take(&mut self, bundle: Bundle, sharers: &[usize], batch: &mut BinaryWriter) -> NextBatch {
        let share = MAX_BATCH_BYTES / sharers.len();
        let mut slices = std::mem::take(&mut self.slices);

        slices.clear();
        for &index in sharers {
            self.outboxes[index].plan(bundle, share, &mut slices, index);
        }

        let outcome = self.write_batch(&slices, batch);

        for slice in &slices {
            self.pending_bytes -= slice.end - slice.start;

            let outbox = &mut self.outboxes[slice.outbox];

            match slice.piece {
                Piece::Frames { .. } => outbox.take(slice.end),
                Piece::Fragment { ends } => outbox.commit_fragment(slice.end, ends),
            }
        }

        self.slices = slices;
        self.aliases.reclaim_aliases();

        match outcome {
            Ok(start) => NextBatch::Ready { start },
            Err(_) => NextBatch::Failed,
        }
    }

    pub(super) fn clear(&mut self) {
        self.outboxes.clear();
        self.index.clear();
        self.last = None;
        self.ready.clear();
        self.pending_bytes = 0;
        self.aliases.clear();
    }

    fn outbox(&mut self, from: Role, channel: u32) -> usize {
        let key = OutboxKey {
            role: from,
            channel,
        };

        if let Some((last, index)) = self.last
            && last == key
        {
            return index;
        }

        let reserved_prefix = self.capacity;
        let outboxes = &mut self.outboxes;
        let index = *self.index.entry(key).or_insert_with(|| {
            outboxes.push(Outbox::new(from, channel, reserved_prefix));

            outboxes.len() - 1
        });

        self.last = Some((key, index));

        index
    }

    fn write_batch(
        &mut self,
        slices: &[PlannedSlice],
        batch: &mut BinaryWriter,
    ) -> Result<usize, Error> {
        if let [slice] = slices
            && let Piece::Frames { last } = slice.piece
            && self.outboxes[slice.outbox].is_all(slice)
        {
            return self.write_in_place(slice, last, batch);
        }

        let count = u32::try_from(slices.len()).expect("fewer than 2^32 slices");
        let mut writer =
            BatchWriter::with_reserved_prefix(batch, self.capacity, CORE_MAJOR, count)?;

        for slice in slices {
            let outbox = &self.outboxes[slice.outbox];
            let key = self.aliases.key(outbox.route(slice.connection));
            let frames = &outbox.frames.as_slice()[slice.start..slice.end];

            match slice.piece {
                Piece::Frames { last } => writer.slice(key, frames, last - slice.start)?,
                Piece::Fragment { ends } => writer.fragment(key, frames, ends)?,
            }
        }

        Ok(self.capacity)
    }

    /// Writes the header and key into the reserved prefix of the outbox buffer covered by `slice`, then replaces it
    /// with `batch`'s buffer for future frames.
    fn write_in_place(
        &mut self,
        slice: &PlannedSlice,
        last: usize,
        batch: &mut BinaryWriter,
    ) -> Result<usize, Error> {
        let outbox = &mut self.outboxes[slice.outbox];
        let key = self.aliases.key(outbox.route(slice.connection));

        self.header.clear();
        BatchWriter::with_reserved_prefix(&mut self.header, 0, CORE_MAJOR, 1)?.key(key)?;

        let start = slice.start - self.header.len();

        debug_assert!(
            start >= self.capacity,
            "the retained prefix fits a batch's header and key"
        );

        let frames = outbox.frames.bytes_mut();
        let length_bytes = BatchWriter::length_bytes(&frames[last + 1..])?;

        frames[last] |= LAST_FRAME;
        frames.copy_within(last + 1 + length_bytes..slice.end, last + 1);
        frames.truncate(slice.end - length_bytes);
        frames[start..slice.start].copy_from_slice(self.header.as_slice());

        let spare = std::mem::take(batch.bytes_mut());

        *batch.bytes_mut() = std::mem::replace(frames, spare);

        Ok(start)
    }
}

#[cfg(test)]
#[path = "tests/outbox.rs"]
mod tests;
