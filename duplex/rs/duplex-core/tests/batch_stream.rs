use duplex_core::{BatchStream, Error, InboundBufferManager};

use crate::support::batch_stream::{batches_of, streamed};

#[test]
fn should_write_a_batch_after_its_length() {
    let bytes = streamed(&[&[7, 8, 9]]);

    assert_eq!(bytes, [3, 7, 8, 9]);
}

#[test]
fn should_write_a_long_batch_after_its_length_as_a_varuint() {
    let bytes = streamed(&[&[0; 300]]);

    assert_eq!(bytes[..2], [0xac, 0x02]);
    assert_eq!(bytes.len(), 302);
}

#[test]
fn should_read_back_the_batches_of_one_read() {
    let bytes = streamed(&[&[1], &[], &[2, 3], &[4; 200]]);
    let batches = batches_of(&mut BatchStream::default(), &bytes);

    assert_eq!(batches, Ok(vec![vec![1], vec![], vec![2, 3], vec![4; 200]]));
}

#[test]
fn should_wait_for_the_rest_of_a_batch_split_across_reads() {
    let bytes = streamed(&[&[1, 2, 3], &[4; 130]]);
    let mut stream = BatchStream::default();
    let reads: Vec<_> = bytes
        .chunks(3)
        .map(|chunk| {
            batches_of(&mut stream, chunk).expect("every chunk of the split batches is processed")
        })
        .collect();
    let batches: Vec<_> = reads.iter().flatten().cloned().collect();

    assert_eq!(batches, [vec![1, 2, 3], vec![4; 130]]);
    assert_eq!(reads[1], [vec![1, 2, 3]]);
}

#[test]
fn should_wait_for_the_rest_of_a_length_split_across_reads() {
    let bytes = streamed(&[&[5; 200]]);
    let mut stream = BatchStream::default();
    let first = batches_of(&mut stream, &bytes[..1]).expect("the first length byte is processed");
    let rest = batches_of(&mut stream, &bytes[1..])
        .expect("the remaining length and batch bytes are processed");

    assert_eq!(first, Vec::<Vec<u8>>::new());
    assert_eq!(rest, [vec![5; 200]]);
}

#[test]
fn should_reject_a_length_past_the_longest_batch() {
    let batches = batches_of(&mut BatchStream::default(), &[0xff, 0xff, 0xff, 0x7f]);

    assert!(matches!(batches, Err(Error::Decode(_))));
}

#[test]
fn should_reject_a_length_of_more_than_four_bytes() {
    let batches = batches_of(&mut BatchStream::default(), &[0x80, 0x80, 0x80, 0x80, 0x01]);

    assert!(matches!(batches, Err(Error::Decode(_))));
}

#[test]
fn should_hand_out_batches_without_copying_them_and_read_into_the_same_buffer_once_free() {
    let mut stream = BatchStream::default();
    let first_at = {
        let mut first = None;

        stream
            .process(&streamed(&[&[1, 2]]), |batch| first = Some(batch))
            .expect("the batch is read");

        first.expect("the batch is handed out").as_slice().as_ptr()
    };
    let mut second = None;

    stream
        .process(&streamed(&[&[3, 4]]), |batch| second = Some(batch))
        .expect("the batch is read");

    let second = second.expect("the batch is handed out");

    assert_eq!(second.as_slice(), [3, 4]);
    assert_eq!(second.as_slice().as_ptr(), first_at);
}

#[test]
fn should_read_into_another_buffer_while_a_batch_is_held() {
    let mut stream = BatchStream::default();
    let mut held = None;

    stream
        .process(&streamed(&[&[1; 100]]), |batch| held = Some(batch))
        .expect("the batch is read");

    let held = held.expect("the batch is handed out");

    let mut later = Vec::new();

    for value in 2..=3 {
        stream
            .process(&streamed(&[vec![value; 40_000].as_slice()]), |batch| {
                later.push(batch.as_slice().to_vec());
            })
            .expect("the batch is read");
    }

    assert_eq!(held.as_slice(), [1; 100]);
    assert_eq!(later, [vec![2; 40_000], vec![3; 40_000]]);
}

#[test]
fn should_read_a_batch_longer_than_one_read() {
    let bytes = streamed(&[vec![9; 200_000].as_slice(), &[8]]);
    let mut stream = BatchStream::default();
    let mut batches = Vec::new();

    for chunk in bytes.chunks(50_000) {
        stream
            .process(chunk, |batch| batches.push(batch.as_slice().to_vec()))
            .expect("every chunk is read");
    }

    assert_eq!(batches, [vec![9; 200_000], vec![8]]);
}

#[test]
fn should_split_whole_batches_sharing_their_buffer() {
    let whole = InboundBufferManager::default().copy(&streamed(&[&[1], &[], &[2; 300]]));
    let mut batches = Vec::new();

    BatchStream::split(&whole, |batch| batches.push(batch)).expect("the batches split");

    let bytes: Vec<_> = batches
        .iter()
        .map(|batch| batch.as_slice().to_vec())
        .collect();

    assert_eq!(bytes, [vec![1], vec![], vec![2; 300]]);
    assert_eq!(
        batches[0].as_slice().as_ptr(),
        whole.as_slice()[1..].as_ptr()
    );
}

#[test]
fn should_refuse_to_split_batches_ending_inside_one() {
    let mut cut = streamed(&[&[1, 2, 3]]);

    cut.pop();

    let whole = InboundBufferManager::default().copy(&cut);

    let split = BatchStream::split(&whole, |_| {});

    assert!(split.is_err());
}

#[test]
fn should_read_little_at_first_and_more_after_each_read_that_filled_its_room() {
    let mut stream = BatchStream::default();
    let mut sizes = Vec::new();

    for _ in 0..6 {
        let space = stream.take_remains();

        sizes.push(space.len());

        let count = space.len();

        space.fill(0);
        stream
            .read(count, |_| {})
            .expect("the empty batches are read");
    }

    assert_eq!(sizes, [4096, 8192, 16_384, 32_768, 65_536, 65_536]);
}

#[test]
fn should_keep_reading_little_while_its_reads_leave_room() {
    let mut stream = BatchStream::default();
    let batch = streamed(&[&[7; 100]]);
    let mut sizes = Vec::new();

    for _ in 0..3 {
        let space = stream.take_remains();

        sizes.push(space.len());
        space[..batch.len()].copy_from_slice(&batch);
        stream.read(batch.len(), |_| {}).expect("the batch is read");
    }

    assert!(sizes.iter().all(|&size| size <= 4096), "{sizes:?}");
}
