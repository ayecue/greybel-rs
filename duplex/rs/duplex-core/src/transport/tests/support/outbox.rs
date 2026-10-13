use super::super::{FlushState, NextBatch, Outbox, OutboxContainer};
use super::*;
use crate::binary::BinaryWriter;

impl Outbox {
    fn unsent_len(&self) -> usize {
        self.frames.len() - self.unsent_start
    }
}

impl OutboxContainer {
    pub(super) fn take_one(&mut self, batch: &mut BinaryWriter) -> bool {
        let mut pass = FlushState::default();

        self.begin_flush(&mut pass);

        let next = self.take_next(&mut pass, batch);

        if let NextBatch::Ready { start } = next {
            batch.remove_front(start);
        }

        self.end_flush();

        matches!(next, NextBatch::Ready { .. })
    }

    pub(super) fn unsent_len(&self) -> usize {
        self.outboxes.iter().map(Outbox::unsent_len).sum()
    }
}

pub const ROUTE: Route = Route {
    from: Role::Client,
    channel: 1,
    connection: None,
};

pub const OTHER: Route = Route {
    from: Role::Client,
    channel: 2,
    connection: None,
};

pub const ANY_SIZE: usize = usize::MAX;

pub fn batches(outboxes: &mut OutboxContainer) -> Vec<Vec<u8>> {
    let mut batch = BinaryWriter::new();
    let mut batches = Vec::new();

    while !outboxes.is_empty() {
        assert!(outboxes.take_one(&mut batch));
        batches.push(batch.as_slice().to_vec());
    }

    batches
}

pub const FRAGMENT: u8 = FrameKind::Fragment as u8;

pub fn payloads_of(batches: &[Vec<u8>], channel: u32) -> Vec<Vec<u8>> {
    let mut payloads = Vec::new();
    let mut fragments = Vec::new();
    let mut aliases = std::collections::HashMap::new();

    for batch in batches {
        for slice in BatchReader::new(batch).expect("the batch reads") {
            let slice = slice.expect("the slice reads");
            let route = match slice.key {
                SliceKey::NoAlias(route) => route,
                SliceKey::Announcement(alias, route) => {
                    aliases.insert(alias, route);

                    route
                }
                SliceKey::Alias(alias) => aliases[&alias],
            };

            if route.channel != channel {
                continue;
            }

            let mut reader = FrameReader::new(batch, slice.frames);

            while let Some(raw) = reader.raw().expect("the frames read") {
                let payload = &batch[raw.payload];

                if raw.kind != FRAGMENT {
                    payloads.push(payload.to_vec());
                    continue;
                }

                fragments.extend_from_slice(&payload[1..]);
                if payload[0] == 1 {
                    let mut frame = FrameReader::new(&fragments, 0..fragments.len());
                    let raw = frame
                        .raw()
                        .expect("the frame reads")
                        .expect("a frame was written");

                    payloads.push(fragments[raw.payload].to_vec());
                    fragments.clear();
                }
            }
        }
    }

    payloads
}

pub fn write(outboxes: &mut OutboxContainer, route: Route, bundle: Bundle, payload: &[u8]) {
    outboxes
        .write(route, bundle, 0, ANY_SIZE, |frames| frames.raw(0, payload))
        .expect("the write fits the transport's buffer");
}

pub fn pass(outboxes: &mut OutboxContainer) -> Vec<Vec<u8>> {
    let mut pass = FlushState::default();
    let mut batch = BinaryWriter::new();
    let mut batches = Vec::new();

    outboxes.begin_flush(&mut pass);
    loop {
        let start = match outboxes.take_next(&mut pass, &mut batch) {
            NextBatch::Ready { start } => start,
            NextBatch::Failed => panic!("the batch is written"),
            NextBatch::Done => break,
        };

        batches.push(batch.as_slice()[start..].to_vec());
    }

    outboxes.end_flush();

    batches
}
