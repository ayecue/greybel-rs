use duplex_core::Bytes;
use greyhack_message_hook_client::protocol::greyhack::{PrintSentClientRpc, SendFileSizeClientRpc};
use greyhack_message_hook_client::protocol::{GreybelNotification, Payload, WindowPidKind};

#[test]
fn should_decode_the_pids_of_a_payload() {
    let payload = Bytes::from_json(&[3, 5]);
    let pids = WindowPidKind::Payload(&payload.0);
    let [listed, unlisted] = [5, 4].map(|pid| pids.contains(pid));

    assert!(listed);
    assert!(!unlisted);
}

#[test]
fn should_list_the_pids_of_a_payload() {
    let payload = Bytes::from_json(&[3, 5]);
    let pids = WindowPidKind::Payload(&payload.0);
    let listed = pids.to_vec();

    assert_eq!(listed, [3, 5]);
}

#[test]
fn should_contain_nothing_without_a_window() {
    let pids = WindowPidKind::None;
    let contained = pids.contains(-10);

    assert!(!contained);
}

#[test]
fn should_contain_a_pid_of_its_list() {
    let pids = WindowPidKind::List(vec![1, 2]);
    let contained = pids.contains(2);

    assert!(contained);
}

#[test]
fn should_find_the_window_of_a_message_by_its_tagged_field() {
    let print = GreybelNotification::PrintSentClientRpc(PrintSentClientRpc {
        zip_output: Bytes::default(),
        replace_text: false,
        window_pid: 12,
    });
    let pids = print.window_pids();

    assert_eq!(pids, WindowPidKind::One(12));
    assert!(print.matches_pid(12));
}

#[test]
fn should_read_the_windows_of_a_message_from_its_tagged_payload() {
    let size = GreybelNotification::SendFileSizeClientRpc(SendFileSizeClientRpc {
        size: 1,
        mult_speed_disk: 1,
        pid_running: Bytes::from_json(&[7, 8]),
        window_pid: 3,
    });
    let pids = size.window_pids().to_vec();

    assert_eq!(pids, [7, 8]);
}
