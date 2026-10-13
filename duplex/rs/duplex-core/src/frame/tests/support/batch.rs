use super::*;
use crate::binary::BinaryWriter;
use crate::definition::FrameKind;
use crate::frame::{FrameEncoder, FrameReader, SliceKey};

pub const CORE_FOLDER: &str = concat!(env!("DUPLEX"), "/core");

pub const FROZEN: [(&str, u32); 4] = [
    ("BatchHeader", 1_591_356_387),
    ("Route", 3_445_261_949),
    ("Open", 2_723_582_384),
    ("Refuse", 3_895_813_661),
];

pub fn versions() -> duplex_protocol::VersionHistory {
    duplex_test_support::generated::versions(CORE_FOLDER)
}

pub type RawFrame = (u8, Vec<u8>);

pub fn slices(bytes: &[u8]) -> Vec<(SliceKey, Vec<RawFrame>)> {
    BatchReader::new(bytes)
        .expect("the batch to read")
        .map(|slice| {
            let slice = slice.expect("the slice to read");
            let mut frames = FrameReader::new(bytes, slice.frames);
            let mut read = Vec::new();

            while let Some(frame) = frames.raw().expect("the frames to read") {
                read.push((frame.kind, bytes[frame.payload].to_vec()));
            }

            (slice.key, read)
        })
        .collect()
}

pub fn frames(frames: &[(FrameKind, &[u8])]) -> (Vec<u8>, usize) {
    let mut writer = BinaryWriter::new();
    let mut encoder = FrameEncoder::new(&mut writer, BinaryWriter::NEWEST);

    for (kind, payload) in frames {
        encoder
            .raw(kind.byte(), payload)
            .expect("the frame is written");
    }

    let last = encoder.last().expect("a frame was written");

    (writer.into_bytes(), last)
}
