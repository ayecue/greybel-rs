use std::rc::Rc;

use super::{BatchPool, OutboundBatch};

#[test]
fn should_put_a_prefix_in_the_room_left_before_the_batch() {
    let pool = Rc::new(BatchPool::default());
    let mut batch = OutboundBatch::new(vec![0, 0, 0, 7, 8], 3, &pool);
    let at = batch.as_slice().as_ptr();

    batch.prepend(&[1, 2]);
    assert_eq!(batch.as_slice(), [1, 2, 7, 8]);
    assert_eq!(batch.as_slice()[2..].as_ptr(), at);
}

#[test]
fn should_move_the_batch_for_a_prefix_longer_than_its_room() {
    let mut batch = OutboundBatch::from(vec![7, 8]);

    batch.prepend(&[1, 2]);
    assert_eq!(batch.as_slice(), [1, 2, 7, 8]);
}

#[test]
fn should_give_its_buffer_back_to_its_transport_when_dropped() {
    let pool = Rc::new(BatchPool::default());
    let bytes = Vec::with_capacity(64);
    let at = bytes.as_ptr();

    drop(OutboundBatch::new(bytes, 0, &pool));

    let taken = pool.take(0);

    assert_eq!(taken.as_ptr(), at);
}

#[test]
fn should_keep_its_buffer_once_made_a_vec() {
    let pool = Rc::new(BatchPool::default());
    let batch = OutboundBatch::new(vec![0, 7, 8], 1, &pool);

    assert_eq!(batch.into_vec(), [7, 8]);
    assert_eq!(pool.take(0).capacity(), 0);
}
