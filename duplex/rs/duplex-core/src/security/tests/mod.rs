#[path = "support/mod.rs"]
mod support;

use support::{BUFFER, EPHEMERAL, NO_IDENTITY, PROLOGUE, STATIC, read, snow, write};

use super::handshake::{Handshake, Step};
use super::noise::PROTOCOL_NAME;
use super::{Security, SecurityRole};

#[test]
fn should_complete_a_handshake_snow_initiates() {
    let security = Security::new(SecurityRole::Responder);
    let mut ours = Handshake::new(SecurityRole::Responder, PROLOGUE, STATIC, EPHEMERAL);
    let mut theirs = snow(SecurityRole::Initiator);
    let first = write(&mut theirs, &[]);
    let Ok(Step::Send(second)) = ours.read(&security, &first) else {
        panic!("replies to the first message");
    };
    let proof = read(&mut theirs, &second);
    let third = write(&mut theirs, &NO_IDENTITY);
    let Ok(Step::Complete(None, mut established)) = ours.read(&security, &third) else {
        panic!("ends with the third message");
    };

    assert_eq!(proof, NO_IDENTITY);

    let mut transport = theirs
        .into_transport_mode()
        .expect("the handshake is complete");
    let mut sent = vec![0; BUFFER];
    let length = transport
        .write_message(b"to the responder", &mut sent)
        .expect("snow encrypts the batch");

    assert_eq!(
        established
            .receive_cipher
            .decrypt(&[], &sent[..length])
            .as_deref(),
        Some(b"to the responder".as_slice())
    );

    let reply = established.send_cipher.encrypt(&[], b"to the initiator");
    let mut received = vec![0; BUFFER];
    let length = transport
        .read_message(&reply, &mut received)
        .expect("snow decrypts the batch");

    assert_eq!(&received[..length], b"to the initiator");
}

#[test]
fn should_complete_a_handshake_snow_responds_to() {
    let security = Security::new(SecurityRole::Initiator);
    let mut ours = Handshake::new(SecurityRole::Initiator, PROLOGUE, STATIC, EPHEMERAL);
    let mut theirs = snow(SecurityRole::Responder);
    let first = ours.start();

    read(&mut theirs, &first);

    let second = write(&mut theirs, &NO_IDENTITY);
    let Ok(Step::Complete(Some(third), mut established)) = ours.read(&security, &second) else {
        panic!("replies to the second message, and ends");
    };
    let proof = read(&mut theirs, &third);

    assert_eq!(proof, NO_IDENTITY);

    let mut transport = theirs
        .into_transport_mode()
        .expect("the handshake is complete");
    let sent = established.send_cipher.encrypt(&[], b"to the responder");
    let mut received = vec![0; BUFFER];
    let length = transport
        .read_message(&sent, &mut received)
        .expect("snow decrypts the batch");

    assert_eq!(&received[..length], b"to the responder");
}
