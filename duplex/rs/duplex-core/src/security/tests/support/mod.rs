use super::*;

pub const PROLOGUE: &[u8] = b"duplex test";

pub const STATIC: [u8; 32] = [7; 32];

pub const EPHEMERAL: [u8; 32] = [8; 32];

pub const NO_IDENTITY: [u8; 1] = [0];

pub const BUFFER: usize = 1024;

pub fn snow(role: SecurityRole) -> snow::HandshakeState {
    let name = std::str::from_utf8(PROTOCOL_NAME).expect("the Noise protocol name is ASCII");
    let builder = snow::Builder::new(name.parse().expect("the Noise protocol name parses"))
        .local_private_key(&[9; 32])
        .expect("snow accepts the static key")
        .prologue(PROLOGUE)
        .expect("snow accepts the prologue")
        .fixed_ephemeral_key_for_testing_only(&[10; 32]);

    match role {
        SecurityRole::Initiator => builder.build_initiator(),
        SecurityRole::Responder => builder.build_responder(),
    }
    .expect("snow builds the handshake")
}

pub fn write(state: &mut snow::HandshakeState, payload: &[u8]) -> Vec<u8> {
    let mut message = vec![0; BUFFER];
    let length = state
        .write_message(payload, &mut message)
        .expect("snow writes the handshake message");

    message.truncate(length);

    message
}

pub fn read(state: &mut snow::HandshakeState, message: &[u8]) -> Vec<u8> {
    let mut payload = vec![0; BUFFER];
    let length = state
        .read_message(message, &mut payload)
        .expect("snow reads the handshake message");

    payload.truncate(length);

    payload
}
