mod handshake;
mod noise;

use handshake::{Handshake, HandshakeOutcome, HandshakeRefusal, SIGNATURE_LEN, Step};
use noise::KEY_LEN;

use crate::binary::BinaryReader;
use crate::frame::CORE_MAJOR;
use crate::protocol::SecurityLevel;

const PROLOGUE: &[u8] = b"duplex";
const BATCH_CONTEXT: &[u8] = b"duplex batch";
const NO_SLICES: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Peer role in the Noise handshake.
pub enum SecurityRole {
    /// The initiator emits the opening handshake message.
    Initiator,
    /// The responder answers the initiator's opening handshake message.
    Responder,
}

pub struct Identity(ed25519_dalek::SigningKey);

/// Configuration for Noise XX handshakes, batch encryption, identity authentication, and batch signatures.
pub struct Security {
    role: SecurityRole,
    identity: Option<Identity>,
    sign_batches: bool,
    verify: Option<Verify>,
    entropy: Entropy,
}

type Verify = Box<dyn Fn(&[u8; KEY_LEN]) -> bool>;

type Entropy = Box<dyn Fn(&mut [u8]) -> Result<(), EntropyUnavailable>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Secure operating-system randomness was unavailable, preventing transport security.
pub struct EntropyUnavailable;

impl Security {
    #[must_use]
    /// Creates `role` security configuration with operating-system random keys.
    pub fn new(role: SecurityRole) -> Self {
        Self {
            role,
            identity: None,
            sign_batches: false,
            verify: None,
            entropy: Box::new(|bytes| getrandom::fill(bytes).map_err(|_| EntropyUnavailable)),
        }
    }

    /// Generates keys from `entropy` instead of operating-system randomness.
    ///
    /// Use only for tests.
    #[must_use]
    pub fn with_entropy(
        mut self,
        entropy: impl Fn(&mut [u8]) -> Result<(), EntropyUnavailable> + 'static,
    ) -> Self {
        self.entropy = Box::new(entropy);

        self
    }

    #[must_use]
    /// Sets the Ed25519 secret key for authenticating this peer.
    pub fn with_identity(mut self, secret: [u8; KEY_LEN]) -> Self {
        self.identity = Some(Identity::of(secret));

        self
    }

    #[must_use]
    /// Requires identity authentication and accepts only remote identities approved by `verify`.
    pub fn with_verifier(mut self, verify: impl Fn(&[u8; KEY_LEN]) -> bool + 'static) -> Self {
        self.verify = Some(Box::new(verify));

        self
    }

    #[must_use]
    /// Accepts only the listed remote Ed25519 public keys.
    pub fn with_pinned_keys(self, keys: Vec<[u8; KEY_LEN]>) -> Self {
        self.with_verifier(move |key| keys.contains(key))
    }

    #[must_use]
    /// Signs every batch with the identity key. Both peers must enable signing.
    pub fn with_batch_signatures(mut self) -> Self {
        self.sign_batches = true;

        self
    }

    #[must_use]
    /// Returns [`SecurityLevel::Authentication`] with a verifier configured, or [`SecurityLevel::Encryption`]
    /// otherwise.
    pub fn level(&self) -> SecurityLevel {
        if self.verify.is_some() {
            SecurityLevel::Authentication
        } else {
            SecurityLevel::Encryption
        }
    }

    /// Returns whether batch signing is enabled.
    #[must_use]
    pub fn signs_batches(&self) -> bool {
        self.sign_batches
    }

    /// Derives the Ed25519 public key from `secret`.
    #[must_use]
    pub fn public_key(secret: [u8; KEY_LEN]) -> [u8; KEY_LEN] {
        ed25519_dalek::SigningKey::from_bytes(&secret)
            .verifying_key()
            .to_bytes()
    }
}

pub(crate) enum ReceiveOutcome {
    Batch(Vec<u8>),
    Reply(Vec<u8>),
    Nothing,
    Failure,
}

enum State {
    Handshake(Handshake),
    Ready(HandshakeOutcome),
    Failure,
}

pub(crate) struct Secure {
    security: Security,
    state: State,
}

impl Secure {
    pub(crate) fn new(security: Security) -> Result<(Self, Option<Vec<u8>>), EntropyUnavailable> {
        let key = || {
            let mut key = [0; KEY_LEN];

            (security.entropy)(&mut key).map(|()| key)
        };
        let (static_key, ephemeral) = (key()?, key()?);
        let prologue = [PROLOGUE, &[CORE_MAJOR]].concat();
        let mut handshake = Handshake::new(security.role, &prologue, static_key, ephemeral);
        let first = (security.role == SecurityRole::Initiator)
            .then(|| Self::handshake_batch(&handshake.start()));
        let secure = Self {
            security,
            state: State::Handshake(handshake),
        };

        Ok((secure, first))
    }

    pub(crate) fn is_established(&self) -> bool {
        matches!(self.state, State::Ready(_))
    }

    pub(crate) fn seal(&mut self, batch: &[u8]) -> Option<Vec<u8>> {
        let State::Ready(outcome) = &mut self.state else {
            return None;
        };
        let header = Self::header_len(batch)?;
        let (header, body) = batch.split_at(header);
        let mut envelope = header.to_vec();

        envelope.extend(outcome.send_cipher.encrypt(header, body));
        if outcome.signs_batches {
            let identity = self.security.identity.as_ref()?;
            let signature = identity.sign(&[BATCH_CONTEXT, &envelope].concat());

            envelope.extend(signature);
        }

        Some(envelope)
    }

    pub(crate) fn receive(&mut self, envelope: &[u8]) -> ReceiveOutcome {
        let Some(header) = Self::header_len(envelope) else {
            return ReceiveOutcome::Nothing;
        };

        if envelope[header - 1] == NO_SLICES && header == 2 {
            return self.handshake(&envelope[header..]);
        }

        let State::Ready(outcome) = &mut self.state else {
            return ReceiveOutcome::Nothing;
        };
        let mut ciphertext = &envelope[header..];

        if outcome.signs_batches {
            let Some(remote) = &outcome.remote else {
                return self.fail();
            };
            let Some(split) = envelope.len().checked_sub(SIGNATURE_LEN) else {
                return self.fail();
            };
            let signature_input = [BATCH_CONTEXT, &envelope[..split]].concat();

            if split < header || !Identity::verifies(remote, &signature_input, &envelope[split..]) {
                return self.fail();
            }

            ciphertext = &envelope[header..split];
        }

        match outcome
            .receive_cipher
            .decrypt(&envelope[..header], ciphertext)
        {
            Some(body) => ReceiveOutcome::Batch([&envelope[..header], &body].concat()),
            None => self.fail(),
        }
    }

    fn handshake(&mut self, message: &[u8]) -> ReceiveOutcome {
        let State::Handshake(handshake) = &mut self.state else {
            return ReceiveOutcome::Nothing;
        };

        match handshake.read(&self.security, message) {
            Ok(Step::Send(reply)) => ReceiveOutcome::Reply(Self::handshake_batch(&reply)),
            Ok(Step::Complete(reply, outcome)) => {
                self.state = State::Ready(*outcome);

                reply.map_or(ReceiveOutcome::Nothing, |reply| {
                    ReceiveOutcome::Reply(Self::handshake_batch(&reply))
                })
            }
            Err(HandshakeRefusal::Invalid | HandshakeRefusal::UnknownIdentity) => self.fail(),
        }
    }

    fn fail(&mut self) -> ReceiveOutcome {
        self.state = State::Failure;

        ReceiveOutcome::Failure
    }

    fn handshake_batch(message: &[u8]) -> Vec<u8> {
        [&[CORE_MAJOR, NO_SLICES], message].concat()
    }

    fn header_len(batch: &[u8]) -> Option<usize> {
        let mut reader = BinaryReader::new(batch.get(1..)?);

        reader.u32().ok()?;

        Some(1 + reader.position())
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
