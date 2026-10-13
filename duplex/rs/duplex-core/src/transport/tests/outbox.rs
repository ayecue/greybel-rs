use support::{ANY_SIZE, FRAGMENT, OTHER, ROUTE, batches, pass, payloads_of, write};

#[path = "support/outbox.rs"]
mod support;

use super::{MAX_BATCH_BYTES, MAX_BUFFERED_BYTES, MIN_FRAGMENT_BYTES, OutboxContainer};
use crate::binary::BinaryWriter;
use crate::channel::Bundle;
use crate::definition::{ErrorCode, FrameKind, Role};
use crate::error::Error;
use crate::frame::{BatchReader, FrameReader, Route, SliceKey};

#[test]
fn should_refuse_writes_once_it_holds_the_most_it_holds() {
    let mut outboxes = OutboxContainer::default();
    let most = vec![0; MAX_BUFFERED_BYTES];

    write(&mut outboxes, ROUTE, Bundle::Common, &most);

    let before = outboxes.unsent_len();

    let written = outboxes.write(ROUTE, Bundle::Common, 0, ANY_SIZE, |frames| {
        frames.raw(0, &[1])
    });

    assert_eq!(written, Err(Error::Rejection(ErrorCode::Overload)));
    assert_eq!(outboxes.unsent_len(), before);
}

#[test]
fn should_write_nothing_of_a_frame_past_the_most_a_frame_takes() {
    let mut outboxes = OutboxContainer::default();

    let written = outboxes.write(ROUTE, Bundle::Common, 0, 100, |frames| {
        frames.raw(0, &[0; 100])
    });

    assert_eq!(written, Err(Error::Rejection(ErrorCode::Overload)));
    assert!(outboxes.is_empty());
}

#[test]
fn should_send_what_waits_past_a_full_batch_in_the_next_by_whole_frames() {
    let mut outboxes = OutboxContainer::default();
    let frame = vec![7; 1000];

    for _ in 0..200 {
        write(&mut outboxes, ROUTE, Bundle::Common, &frame);
    }

    let batches = batches(&mut outboxes);

    let mut frames = 0;

    for batch in &batches {
        for slice in BatchReader::new(batch).expect("the batch reads") {
            let slice = slice.expect("the slice reads");
            let mut reader = FrameReader::new(batch, slice.frames);

            while let Some(raw) = reader.raw().expect("the frames read") {
                assert_ne!(raw.kind, FRAGMENT);
                assert_eq!(&batch[raw.payload], &frame[..]);
                frames += 1;
            }
        }
    }

    assert_eq!(frames, 200);
    assert_eq!(batches.len(), 4);
    assert!(
        batches
            .iter()
            .all(|batch| batch.len() <= MAX_BATCH_BYTES + 16)
    );
}

#[test]
fn should_send_a_frame_past_a_batch_in_fragments_that_fit_one() {
    let mut outboxes = OutboxContainer::default();
    let frame: Vec<u8> = (0..=u8::MAX).cycle().take(300_000).collect();

    write(&mut outboxes, ROUTE, Bundle::Common, &frame);

    let batches = batches(&mut outboxes);

    assert_eq!(batches.len(), 300_000 / MAX_BATCH_BYTES + 1);
    assert!(
        batches
            .iter()
            .all(|batch| batch.len() <= MAX_BATCH_BYTES + 16)
    );
    assert_eq!(payloads_of(&batches, ROUTE.channel), [frame]);
}

#[test]
fn should_share_batches_with_other_channels_while_a_frame_goes_in_fragments() {
    let mut outboxes = OutboxContainer::default();
    let large = vec![1; 4 * MAX_BATCH_BYTES];
    let small = vec![2; 100];

    write(&mut outboxes, ROUTE, Bundle::Common, &large);
    write(&mut outboxes, OTHER, Bundle::Common, &small);

    let mut batch = BinaryWriter::new();

    assert!(outboxes.take_one(&mut batch));

    let first = batch.as_slice().to_vec();

    assert_eq!(
        payloads_of(std::slice::from_ref(&first), OTHER.channel),
        [small]
    );
    assert!(first.len() <= MAX_BATCH_BYTES / 2 + MIN_FRAGMENT_BYTES);

    let rest = batches(&mut outboxes);
    let mut all = vec![first];

    all.extend(rest);
    assert_eq!(payloads_of(&all, ROUTE.channel), [large]);
}

#[test]
fn should_keep_the_frames_after_a_fragmented_one_in_their_order() {
    let mut outboxes = OutboxContainer::default();
    let large = vec![3; 2 * MAX_BATCH_BYTES];
    let after = vec![4; 10];

    write(&mut outboxes, ROUTE, Bundle::Common, &large);
    write(&mut outboxes, ROUTE, Bundle::Common, &after);

    let batches = batches(&mut outboxes);

    assert_eq!(payloads_of(&batches, ROUTE.channel), [large, after]);
}

#[test]
fn should_send_nothing_more_of_a_channel_after_its_first_frame_past_its_share() {
    let mut outboxes = OutboxContainer::default();
    let large = vec![5; MAX_BATCH_BYTES / 5 + 1000];
    let later = Route {
        connection: crate::connection_id::ConnectionId::of(7),
        ..ROUTE
    };

    write(&mut outboxes, ROUTE, Bundle::Common, &large);
    write(&mut outboxes, later, Bundle::Common, &[6; 10]);
    for channel in 2..=5 {
        let route = Route { channel, ..OTHER };

        write(&mut outboxes, route, Bundle::Common, &[7; 10]);
    }

    let mut batch = BinaryWriter::new();

    assert!(outboxes.take_one(&mut batch));

    assert_eq!(
        payloads_of(&[batch.as_slice().to_vec()], ROUTE.channel),
        [large]
    );
}

#[test]
fn should_send_a_batch_of_each_bundle_in_one_pass_in_the_order_written() {
    let mut outboxes = OutboxContainer::default();

    for key in 1..=3u8 {
        write(&mut outboxes, ROUTE, Bundle::Key(key.into()), &[key]);
        write(&mut outboxes, OTHER, Bundle::Key(key.into()), &[10 + key]);
    }

    let batches = pass(&mut outboxes);

    assert_eq!(batches.len(), 3);
    assert!(outboxes.is_empty());
    for (key, batch) in (1..=3u8).zip(&batches) {
        let batch = std::slice::from_ref(batch);

        assert_eq!(payloads_of(batch, ROUTE.channel), [[key]]);
        assert_eq!(payloads_of(batch, OTHER.channel), [[10 + key]]);
    }
}

#[test]
fn should_send_what_a_channel_wrote_after_another_bundle_in_the_next_pass() {
    let mut outboxes = OutboxContainer::default();

    write(&mut outboxes, ROUTE, Bundle::Common, &[1]);
    write(&mut outboxes, ROUTE, Bundle::Key(1), &[2]);
    write(&mut outboxes, ROUTE, Bundle::Common, &[3]);

    let first = pass(&mut outboxes);
    let second = pass(&mut outboxes);

    assert_eq!(payloads_of(&first, ROUTE.channel), [[1], [2]]);
    assert_eq!(first.len(), 2);
    assert_eq!(payloads_of(&second, ROUTE.channel), [[3]]);
    assert!(outboxes.is_empty());
}
