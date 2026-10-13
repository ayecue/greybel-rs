use duplex_core::InboundBufferManager;

#[test]
fn should_hand_out_a_copy_of_what_came_in() {
    let mut buffers = InboundBufferManager::default();

    let batch = buffers.copy(&[1, 2, 3]);

    assert_eq!(batch.as_slice(), [1, 2, 3]);
}

#[test]
fn should_copy_into_the_same_buffer_once_nothing_holds_its_batch() {
    let mut buffers = InboundBufferManager::default();
    let first_at = buffers.copy(&[1, 2]).as_slice().as_ptr();

    let second = buffers.copy(&[3, 4]);

    assert_eq!(second.as_slice(), [3, 4]);
    assert_eq!(second.as_slice().as_ptr(), first_at);
}

#[test]
fn should_copy_into_another_buffer_while_a_batch_is_held() {
    let mut buffers = InboundBufferManager::default();
    let held = buffers.copy(&[1; 100]);

    let later: Vec<_> = (2..=4).map(|value| buffers.copy(&[value; 10])).collect();

    assert_eq!(held.as_slice(), [1; 100]);

    let later: Vec<_> = later
        .iter()
        .map(|batch| batch.as_slice().to_vec())
        .collect();

    assert_eq!(later, [vec![2; 10], vec![3; 10], vec![4; 10]]);
}

#[test]
fn should_hand_out_a_batch_longer_than_its_buffers() {
    let mut buffers = InboundBufferManager::default();
    let long: Vec<u8> = (0..=u8::MAX).cycle().take(200_000).collect();

    let batch = buffers.copy(&long);

    assert_eq!(batch.as_slice(), long);
}

#[test]
fn should_copy_into_a_larger_buffer_once_one_comes_in_too_large_for_the_one_kept() {
    let mut buffers = InboundBufferManager::default();

    drop(buffers.copy(&[1; 10]));

    let larger_at = buffers.copy(&[2; 10_000]).as_slice().as_ptr();

    let later = buffers.copy(&[3; 10]);

    assert_eq!(later.as_slice(), [3; 10]);
    assert_eq!(later.as_slice().as_ptr(), larger_at);
}
