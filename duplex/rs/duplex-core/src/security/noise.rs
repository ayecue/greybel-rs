use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

pub(super) const PROTOCOL_NAME: &[u8; HASH_LEN] = b"Noise_XX_25519_ChaChaPoly_SHA256";
const HASH_LEN: usize = 32;
pub(super) const KEY_LEN: usize = 32;
pub(super) const TAG_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const COUNTER_OFFSET: usize = 4;

pub(super) type Hash = [u8; HASH_LEN];

#[derive(Clone, Default)]
pub(super) struct CipherState {
    key: Option<[u8; KEY_LEN]>,
    nonce: u64,
}

impl CipherState {
    pub(super) fn of(key: [u8; KEY_LEN]) -> Self {
        Self {
            key: Some(key),
            nonce: 0,
        }
    }

    pub(super) fn encrypt(&mut self, ad: &[u8], plaintext: &[u8]) -> Vec<u8> {
        let Some(key) = &self.key else {
            return plaintext.to_vec();
        };
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Self::nonce(self.nonce);

        self.nonce += 1;

        cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: ad,
                },
            )
            .expect("ChaCha20-Poly1305 encrypts any length")
    }

    pub(super) fn decrypt(&mut self, ad: &[u8], ciphertext: &[u8]) -> Option<Vec<u8>> {
        let Some(key) = &self.key else {
            return Some(ciphertext.to_vec());
        };
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let plaintext = cipher
            .decrypt(
                &Self::nonce(self.nonce),
                Payload {
                    msg: ciphertext,
                    aad: ad,
                },
            )
            .ok()?;

        self.nonce += 1;

        Some(plaintext)
    }

    fn nonce(counter: u64) -> Nonce {
        let mut nonce = [0; NONCE_LEN];

        nonce[COUNTER_OFFSET..].copy_from_slice(&counter.to_le_bytes());

        *Nonce::from_slice(&nonce)
    }
}

pub(super) struct SymmetricState {
    chaining_key: Hash,
    hash: Hash,
    cipher: CipherState,
}

impl SymmetricState {
    pub(super) fn new(prologue: &[u8]) -> Self {
        let mut state = Self {
            chaining_key: *PROTOCOL_NAME,
            hash: *PROTOCOL_NAME,
            cipher: CipherState::default(),
        };

        state.mix_hash(prologue);

        state
    }

    pub(super) fn hash(&self) -> Hash {
        self.hash
    }

    pub(super) fn mix_hash(&mut self, data: &[u8]) {
        let mut hasher = Sha256::new();

        hasher.update(self.hash);
        hasher.update(data);
        self.hash = hasher.finalize().into();
    }

    pub(super) fn mix_key(&mut self, input: &[u8]) {
        let [chaining_key, key] = Self::hkdf(&self.chaining_key, input);

        self.chaining_key = chaining_key;
        self.cipher = CipherState::of(key);
    }

    pub(super) fn encrypt_and_hash(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let ciphertext = self.cipher.encrypt(&self.hash, plaintext);

        self.mix_hash(&ciphertext);

        ciphertext
    }

    pub(super) fn decrypt_and_hash(&mut self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        let plaintext = self.cipher.decrypt(&self.hash, ciphertext)?;

        self.mix_hash(ciphertext);

        Some(plaintext)
    }

    pub(super) fn split(&self) -> (CipherState, CipherState) {
        let [initiator, responder] = Self::hkdf(&self.chaining_key, &[]);

        (CipherState::of(initiator), CipherState::of(responder))
    }

    fn hkdf(chaining_key: &Hash, input: &[u8]) -> [Hash; 2] {
        let temporary = Self::hmac(chaining_key, &[input]);
        let first = Self::hmac(&temporary, &[&[1]]);
        let second = Self::hmac(&temporary, &[&first, &[2]]);

        [first, second]
    }

    fn hmac(key: &Hash, parts: &[&[u8]]) -> Hash {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC takes any key");

        for part in parts {
            mac.update(part);
        }

        mac.finalize().into_bytes().into()
    }
}

pub(super) struct KeyPair {
    secret: StaticSecret,
    public: [u8; KEY_LEN],
}

impl KeyPair {
    pub(super) fn of(secret: [u8; KEY_LEN]) -> Self {
        let secret = StaticSecret::from(secret);
        let public = PublicKey::from(&secret).to_bytes();

        Self { secret, public }
    }

    pub(super) fn public(&self) -> [u8; KEY_LEN] {
        self.public
    }

    pub(super) fn agree(&self, public: &[u8; KEY_LEN]) -> [u8; KEY_LEN] {
        self.secret
            .diffie_hellman(&PublicKey::from(*public))
            .to_bytes()
    }
}
