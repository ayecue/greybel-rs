#[path = "support/backlog.rs"]
mod support;

use support::{LIMIT, after};

use super::Backlog;

#[test]
fn should_keep_each_batch_until_the_client_has_it() {
    let mut retained = Backlog::new(LIMIT);

    for batch in [[1], [2], [3]] {
        retained.push(&batch);
    }

    retained.acknowledge(1);

    assert_eq!(after(&retained, 0), [(2, vec![2]), (3, vec![3])]);
    assert_eq!(after(&retained, 2), [(3, vec![3])]);
    assert_eq!(retained.last(), 3);
}

#[test]
fn should_write_each_batch_as_a_record_of_its_number_and_bytes() {
    let mut retained = Backlog::new(LIMIT);

    retained.push(&[7, 8]);
    for _ in 0..200 {
        retained.push(&[]);
    }

    let (records, last) = retained.records_after(199).expect("it has records");

    assert_eq!(last, 201);
    assert_eq!(records, [2, 0xc8, 0x01, 2, 0xc9, 0x01]);

    let (first, _) = retained.records_after(0).expect("it has records");

    assert_eq!(first[..4], [3, 1, 7, 8]);
}

#[test]
fn should_overload_once_its_client_is_too_far_behind() {
    let mut retained = Backlog::new(10);

    retained.push(&[1, 2, 3]);
    retained.push(&[4, 5]);
    retained.acknowledge(1);

    let accepted = retained.push(&[6; 5]);

    assert!(!accepted);
    assert!(retained.is_overloaded());
    assert_eq!(after(&retained, 0), [(2, vec![4, 5])]);
}

#[test]
fn should_keep_its_records_once_it_moved_what_is_left_to_the_front() {
    let mut retained = Backlog::new(usize::MAX);
    let large = vec![9; 100 * 1024];

    retained.push(&large);
    retained.push(&[1]);
    retained.push(&[2]);

    retained.acknowledge(1);

    assert_eq!(after(&retained, 1), [(2, vec![1]), (3, vec![2])]);

    let (records, last) = retained.records_after(2).expect("it has records");

    assert_eq!((records, last), (&[2, 3, 2][..], 3));
}
