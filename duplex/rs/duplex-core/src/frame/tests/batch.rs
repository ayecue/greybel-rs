#[path = "support/batch.rs"]
mod support;

use support::{FROZEN, frames, slices, versions};

use super::{BatchReader, BatchWriter, CORE_MAJOR, LAST_FRAME};
use crate::binary::BinaryWriter;
use crate::connection_id::ConnectionId;
use crate::definition::{CORE, FrameKind, Role};
use crate::frame::{Route, SliceKey};

#[test]
fn should_speak_the_newest_core() {
    let versions = versions();
    let latest = versions.latest().expect("the core has a version").version;

    assert_eq!((CORE.major, CORE.minor), (latest.major, latest.minor));
}

#[test]
fn should_keep_the_frozen_layouts_in_every_core() {
    let versions = versions();
    let changed: Vec<String> = versions
        .all()
        .iter()
        .flat_map(|version| {
            FROZEN.iter().filter_map(move |&(name, frozen)| {
                let fingerprint = version.fingerprint_of(name);

                (fingerprint != Some(frozen))
                    .then(|| format!("{} `{name}`: {fingerprint:?}", version.version))
            })
        })
        .collect();

    assert!(
        changed.is_empty(),
        "these changed, which no core may: {changed:?}"
    );
}

#[test]
fn should_keep_every_frame_kind_below_the_last_frame_bit() {
    let kinds: Vec<FrameKind> = (0..=u32::from(u8::MAX))
        .filter_map(|value| FrameKind::try_from(value).ok())
        .collect();

    assert!(kinds.contains(&FrameKind::Fragment));
    assert!(kinds.iter().all(|kind| kind.byte() & LAST_FRAME == 0));
}

#[test]
fn should_read_the_slices_a_batch_was_written_with() {
    let route = Route {
        from: Role::Client,
        channel: u32::MAX,
        connection: ConnectionId::of(7),
    };
    let (first, first_last) = frames(&[(FrameKind::Ping, &[]), (FrameKind::Close, &[1])]);
    let (second, second_last) = frames(&[(FrameKind::Pong, &[2, 3])]);
    let mut batch = BinaryWriter::new();
    let mut writer = BatchWriter::new(&mut batch, CORE_MAJOR, 2).expect("the batch is written");

    writer
        .slice(SliceKey::Announcement(1, route), &first, first_last)
        .expect("the slice is written");
    writer
        .slice(SliceKey::Alias(1), &second, second_last)
        .expect("the slice is written");

    let ping = FrameKind::Ping.byte();
    let close = FrameKind::Close.byte();
    let pong = FrameKind::Pong.byte();

    assert_eq!(
        slices(batch.as_slice()),
        [
            (
                SliceKey::Announcement(1, route),
                vec![(ping, vec![]), (close, vec![1])]
            ),
            (SliceKey::Alias(1), vec![(pong, vec![2, 3])]),
        ]
    );
}

#[test]
fn should_leave_out_the_lengths_the_batch_implies() {
    let route = Route {
        from: Role::Provider,
        channel: 1,
        connection: None,
    };
    let (frames, last) = frames(&[(FrameKind::Listen, &[])]);
    let mut batch = BinaryWriter::new();

    BatchWriter::new(&mut batch, CORE_MAJOR, 1)
        .and_then(|mut writer| writer.slice(SliceKey::NoAlias(route), &frames, last))
        .expect("the slice is written");

    let listening = FrameKind::Listen.byte();

    assert_eq!(
        batch.as_slice(),
        [
            [CORE_MAJOR, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0].as_slice(),
            &[listening | LAST_FRAME]
        ]
        .concat()
    );
}
