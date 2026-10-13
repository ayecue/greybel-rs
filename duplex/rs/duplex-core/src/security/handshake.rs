use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use super::noise::{CipherState, Hash, KEY_LEN, KeyPair, SymmetricState, TAG_LEN};
use super::{Identity, Security, SecurityRole};

const IDENTITY_FLAG: u8 = 0b01;
const SIGNS_BATCHES: u8 = 0b10;
const FLAGS: u8 = IDENTITY_FLAG | SIGNS_BATCHES;
pub(super) const SIGNATURE_LEN: usize = 64;
const IDENTITY_CONTEXT: &[u8] = b"duplex identity";
const ENCRYPTED_KEY_LEN: usize = KEY_LEN + TAG_LEN;

pub(super) struct HandshakeOutcome {
    pub(super) send_cipher: CipherState,
    pub(super) receive_cipher: CipherState,
    pub(super) remote: Option<VerifyingKey>,
    pub(super) signs_batches: bool,
}

pub(super) enum Step {
    Send(Vec<u8>),
    Complete(Option<Vec<u8>>, Box<HandshakeOutcome>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HandshakeRefusal {
    Invalid,
    UnknownIdentity,
}

pub(super) struct Handshake {
    state: SymmetricState,
    role: SecurityRole,
    static_key: KeyPair,
    ephemeral: KeyPair,
    remote_ephemeral: Option<[u8; KEY_LEN]>,
    message_count: u8,
}

impl Handshake {
    pub(super) fn new(
        role: SecurityRole,
        prologue: &[u8],
        static_key: [u8; KEY_LEN],
        ephemeral: [u8; KEY_LEN],
    ) -> Self {
        Self {
            state: SymmetricState::new(prologue),
            role,
            static_key: KeyPair::of(static_key),
            ephemeral: KeyPair::of(ephemeral),
            remote_ephemeral: None,
            message_count: 0,
        }
    }

    pub(super) fn start(&mut self) -> Vec<u8> {
        let ephemeral = self.ephemeral.public();

        self.state.mix_hash(&ephemeral);

        let mut message = ephemeral.to_vec();

        message.extend(self.state.encrypt_and_hash(&[]));
        self.message_count = 1;

        message
    }

    pub(super) fn read(
        &mut self,
        security: &Security,
        message: &[u8],
    ) -> Result<Step, HandshakeRefusal> {
        match (self.role, self.message_count) {
            (SecurityRole::Responder, 0) => self.read_first(security, message),
            (SecurityRole::Initiator, 1) => self.read_second(security, message),
            (SecurityRole::Responder, 2) => self.read_third(security, message),
            _ => Err(HandshakeRefusal::Invalid),
        }
    }

    fn read_first(
        &mut self,
        security: &Security,
        message: &[u8],
    ) -> Result<Step, HandshakeRefusal> {
        let (remote_ephemeral, payload) = Self::key(message)?;

        self.state.mix_hash(&remote_ephemeral);
        self.state
            .decrypt_and_hash(payload)
            .ok_or(HandshakeRefusal::Invalid)?;
        self.remote_ephemeral = Some(remote_ephemeral);

        let ephemeral = self.ephemeral.public();

        self.state.mix_hash(&ephemeral);
        self.state.mix_key(&self.ephemeral.agree(&remote_ephemeral));

        let mut reply = ephemeral.to_vec();

        reply.extend(self.state.encrypt_and_hash(&self.static_key.public()));
        self.state
            .mix_key(&self.static_key.agree(&remote_ephemeral));

        let identity_payload = Self::build_identity_payload(security, self.state.hash());

        reply.extend(self.state.encrypt_and_hash(&identity_payload));
        self.message_count = 2;

        Ok(Step::Send(reply))
    }

    fn read_second(
        &mut self,
        security: &Security,
        message: &[u8],
    ) -> Result<Step, HandshakeRefusal> {
        let (remote_ephemeral, remainder) = Self::key(message)?;

        self.state.mix_hash(&remote_ephemeral);
        self.state.mix_key(&self.ephemeral.agree(&remote_ephemeral));

        let remote_static = self.read_static(remainder)?;

        self.state.mix_key(&self.ephemeral.agree(&remote_static));

        let remote = self.validate_identity_payload(security, &remainder[ENCRYPTED_KEY_LEN..])?;
        let mut reply = self.state.encrypt_and_hash(&self.static_key.public());

        self.state
            .mix_key(&self.static_key.agree(&remote_ephemeral));

        let identity_payload = Self::build_identity_payload(security, self.state.hash());

        reply.extend(self.state.encrypt_and_hash(&identity_payload));

        let (send_cipher, receive_cipher) = self.state.split();

        Ok(Step::Complete(
            Some(reply),
            Box::new(HandshakeOutcome {
                send_cipher,
                receive_cipher,
                remote,
                signs_batches: security.sign_batches,
            }),
        ))
    }

    fn read_third(
        &mut self,
        security: &Security,
        message: &[u8],
    ) -> Result<Step, HandshakeRefusal> {
        let remote_static = self.read_static(message)?;

        self.state.mix_key(&self.ephemeral.agree(&remote_static));

        let remote = self.validate_identity_payload(security, &message[ENCRYPTED_KEY_LEN..])?;
        let (receive_cipher, send_cipher) = self.state.split();

        Ok(Step::Complete(
            None,
            Box::new(HandshakeOutcome {
                send_cipher,
                receive_cipher,
                remote,
                signs_batches: security.sign_batches,
            }),
        ))
    }

    fn read_static(&mut self, message: &[u8]) -> Result<[u8; KEY_LEN], HandshakeRefusal> {
        let ciphertext = message
            .get(..ENCRYPTED_KEY_LEN)
            .ok_or(HandshakeRefusal::Invalid)?;
        let key = self
            .state
            .decrypt_and_hash(ciphertext)
            .ok_or(HandshakeRefusal::Invalid)?;

        key.try_into().map_err(|_| HandshakeRefusal::Invalid)
    }

    fn validate_identity_payload(
        &mut self,
        security: &Security,
        ciphertext: &[u8],
    ) -> Result<Option<VerifyingKey>, HandshakeRefusal> {
        let signed_hash = self.state.hash();
        let identity_payload = self
            .state
            .decrypt_and_hash(ciphertext)
            .ok_or(HandshakeRefusal::Invalid)?;
        let (&flags, remainder) = identity_payload
            .split_first()
            .ok_or(HandshakeRefusal::Invalid)?;

        if flags & !FLAGS != 0 || (flags & SIGNS_BATCHES != 0) != security.sign_batches {
            return Err(HandshakeRefusal::UnknownIdentity);
        }

        let remote = if flags & IDENTITY_FLAG == 0 {
            None
        } else {
            let (key, signature) = remainder
                .split_at_checked(KEY_LEN)
                .ok_or(HandshakeRefusal::Invalid)?;
            let key = VerifyingKey::try_from(key).map_err(|_| HandshakeRefusal::Invalid)?;
            let signature =
                Signature::from_slice(signature).map_err(|_| HandshakeRefusal::Invalid)?;

            key.verify(&Self::signature_input(&signed_hash), &signature)
                .map_err(|_| HandshakeRefusal::UnknownIdentity)?;

            Some(key)
        };
        let trusted = match (&security.verify, &remote) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(verify), Some(key)) => verify(key.as_bytes()),
        };

        if !trusted || (security.sign_batches && remote.is_none()) {
            return Err(HandshakeRefusal::UnknownIdentity);
        }

        Ok(remote)
    }

    fn build_identity_payload(security: &Security, hash: Hash) -> Vec<u8> {
        let mut flags = 0;

        if security.sign_batches {
            flags |= SIGNS_BATCHES;
        }

        let Some(identity) = &security.identity else {
            return vec![flags];
        };
        let mut identity_payload = vec![flags | IDENTITY_FLAG];

        identity_payload.extend(identity.public());
        identity_payload.extend(identity.sign(&Self::signature_input(&hash)));
        debug_assert_eq!(identity_payload.len(), 1 + KEY_LEN + SIGNATURE_LEN);

        identity_payload
    }

    fn signature_input(hash: &Hash) -> Vec<u8> {
        [IDENTITY_CONTEXT, hash.as_slice()].concat()
    }

    fn key(message: &[u8]) -> Result<([u8; KEY_LEN], &[u8]), HandshakeRefusal> {
        let (key, remainder) = message
            .split_at_checked(KEY_LEN)
            .ok_or(HandshakeRefusal::Invalid)?;

        Ok((
            key.try_into().map_err(|_| HandshakeRefusal::Invalid)?,
            remainder,
        ))
    }
}

impl Identity {
    fn public(&self) -> [u8; KEY_LEN] {
        self.0.verifying_key().to_bytes()
    }

    pub(super) fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN] {
        self.0.sign(message).to_bytes()
    }

    pub(super) fn of(secret: [u8; KEY_LEN]) -> Self {
        Self(SigningKey::from_bytes(&secret))
    }

    pub(super) fn verifies(key: &VerifyingKey, message: &[u8], signature: &[u8]) -> bool {
        Signature::from_slice(signature)
            .is_ok_and(|signature| key.verify(message, &signature).is_ok())
    }
}
