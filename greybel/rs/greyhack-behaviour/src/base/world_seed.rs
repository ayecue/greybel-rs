//! World-secret derivation (`WorldSeed`): keyed numbers and reversible 32-bit permutations, including
//! licence-plate address encoding.

use sha2::{Digest, Sha256};

/// Byte length of the generated world secret.
pub const WORLD_SECRET_LENGTH: usize = 32;

const BLOCK: usize = 64;
const PERMUTATION_KEY: &str = "permute-key-v1";
const ROUNDS: usize = 4;

/// Secret key used for derived numbers and permutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSeed {
    secret: Vec<u8>,
}

impl WorldSeed {
    #[must_use]
    pub fn new(secret: Vec<u8>) -> Self {
        Self { secret }
    }

    #[must_use]
    pub fn secret(&self) -> &[u8] {
        &self.secret
    }

    /// Derives a non-negative integer below `i32::MAX` from `label` and `context` (`WorldSeed.DeriveInt`).
    #[must_use]
    pub fn derive_int(&self, label: &str, context: &[&str]) -> i32 {
        let number = self.derive_u32(label, context) & 0x7fff_ffff;

        i32::try_from(number).map_or(i32::MAX - 1, |number| {
            if number == i32::MAX {
                i32::MAX - 1
            } else {
                number
            }
        })
    }

    /// Derives a 32-bit number from `label` and `context` (`WorldSeed.DeriveUInt32`).
    #[must_use]
    pub fn derive_u32(&self, label: &str, context: &[&str]) -> u32 {
        let hash = self.hmac(label, context);

        u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]])
    }

    /// Derives a 64-bit number from `label` and `context` (`WorldSeed.DeriveUInt64`).
    #[must_use]
    pub fn derive_u64(&self, label: &str, context: &[&str]) -> u64 {
        let hash = self.hmac(label, context);

        u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
        ])
    }

    /// Permutes `value` with four Feistel rounds keyed by `label` (`WorldSeed.PermuteUInt32`).
    #[must_use]
    pub fn permute_u32(&self, label: &str, value: u32) -> u32 {
        let keys = self.permutation_keys(label);
        let (mut left, mut right) = split_u32(value);

        for key in keys {
            (left, right) = (right, left ^ feistel_round(right, key));
        }

        join_u16(left, right)
    }

    /// Recovers the `value` that [`Self::permute_u32`] maps to `permutation` (`WorldSeed.DepermuteUInt32`).
    #[must_use]
    pub fn depermute_u32(&self, label: &str, permutation: u32) -> u32 {
        let keys = self.permutation_keys(label);
        let (mut left, mut right) = split_u32(permutation);

        for key in keys.into_iter().rev() {
            (left, right) = (right ^ feistel_round(left, key), left);
        }

        join_u16(left, right)
    }

    fn permutation_keys(&self, label: &str) -> [u32; ROUNDS] {
        let hash = self.hmac(PERMUTATION_KEY, &[label]);

        std::array::from_fn(|index| {
            let at = index * 4;

            u32::from_be_bytes([hash[at], hash[at + 1], hash[at + 2], hash[at + 3]])
        })
    }

    /// Computes secret-keyed HMAC-SHA256. Each `label` and `context` part is encoded as its UTF-16
    /// length, `:`, its text, and `|` (`WorldSeed.BuildPayload`).
    fn hmac(&self, label: &str, context: &[&str]) -> [u8; 32] {
        let mut payload = String::new();

        for part in std::iter::once(label).chain(context.iter().copied()) {
            payload.push_str(&format!("{}:{part}|", part.encode_utf16().count()));
        }

        let mut key = [0_u8; BLOCK];

        if self.secret.len() > BLOCK {
            key[..32].copy_from_slice(&Sha256::digest(&self.secret));
        } else {
            key[..self.secret.len()].copy_from_slice(&self.secret);
        }

        let pad = |byte: u8| key.map(|key| key ^ byte);
        let inner = Sha256::new()
            .chain_update(pad(0x36))
            .chain_update(payload.as_bytes())
            .finalize();

        Sha256::new()
            .chain_update(pad(0x5c))
            .chain_update(inner)
            .finalize()
            .into()
    }
}

/// Splits `value` into its high and low 16 bits, the two sides of the Feistel network.
fn split_u32(value: u32) -> (u16, u16) {
    ((value >> 16) as u16, value as u16)
}

/// Joins the two sides of the Feistel network back into one value; the inverse of [`split_u32`].
fn join_u16(left: u16, right: u16) -> u32 {
    (u32::from(left) << 16) | u32::from(right)
}

/// Computes the Feistel round function for side `value` and round key `key` (`WorldSeed.GetRoundValue`).
fn feistel_round(value: u16, key: u32) -> u16 {
    let mix = u32::from(value) ^ key;
    let mix = (mix ^ (mix >> 16)).wrapping_mul(2_146_121_005);
    let mix = ((mix ^ (mix >> 15)).cast_signed()).wrapping_mul(-2_073_254_261);

    (mix ^ (mix.cast_unsigned() >> 16).cast_signed()) as u16
}
