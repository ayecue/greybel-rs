//! Persistent SplitMix64 random generator and the alphabets random text is drawn from.
//!
//! The world seeds generators in four ways, each used for one purpose: [`Rng::new`] with a stored
//! state resumes the world generator; [`Rng::fork`] splits a named subsystem generator from it;
//! [`Rng::stable`] rebuilds the generator of one network or device from the world seed and its
//! addresses, so administrator restoration reproduces it; `WorldSeed::derive_*` gives the game's
//! keyed seeds for `mono_random::Random`.

use rand_core::{Rng as _, SeedableRng};
use rand_xoshiro::SplitMix64;

/// Hexadecimal digits in the case the game prints them (`Random.GetMac`).
pub const HEX_DIGITS: &[u8] = b"0123456789ABCDEF";
/// Lowercase letters and digits (wallet PINs, Wi-Fi name suffixes).
pub const LOWERCASE_ALPHANUMERIC: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
/// Upper- and lowercase letters and digits (bank account IDs).
pub const ALPHANUMERIC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;
/// Byte appended after each part of a [`Rng::stable`] key so `["ab", "c"]` and `["a", "bc"]`
/// differ.
const PART_SEPARATOR: u8 = 0xff;

fn fnv1a_extend(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Hashes `text` with 64-bit FNV-1a.
#[must_use]
pub fn fnv1a_64(text: &str) -> u64 {
    fnv1a_extend(FNV_OFFSET, text.as_bytes())
}

/// Persistent SplitMix64 generator whose [`Self::state`] can be stored and resumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng(SplitMix64);

impl Rng {
    /// Creates a generator from `seed`; passing stored [`Self::state`] resumes its sequence.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(SplitMix64::seed_from_u64(seed))
    }

    /// Creates the generator of one entity from the world `seed` and the `parts` that identify it
    /// (addresses and a purpose label), independent of draw order elsewhere.
    #[must_use]
    pub fn stable(seed: u64, parts: &[&str]) -> Self {
        let hash = parts.iter().fold(seed ^ FNV_OFFSET, |hash, part| {
            fnv1a_extend(fnv1a_extend(hash, part.as_bytes()), &[PART_SEPARATOR])
        });

        Self::new(hash)
    }

    /// Creates the generator the shared root password of the network at `public_ip` is drawn from
    /// (`LanRouter`): all of its routers and switches have it, and administrator restoration draws
    /// it again.
    #[must_use]
    pub fn root_password(seed: u64, public_ip: &str) -> Self {
        Self::stable(seed, &[public_ip, crate::ROOT_USER])
    }

    /// Returns current generator state for persistence.
    #[must_use]
    pub fn state(&self) -> u64 {
        u64::from_le_bytes(self.0.state())
    }

    /// Returns the next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    /// Discards the next `draws` draws, for sequences that other draws must not depend on.
    pub fn skip(&mut self, draws: usize) {
        for _ in 0..draws {
            self.next_u64();
        }
    }

    /// Forks an independent subsystem generator so draws in one subsystem do not advance another's
    /// sequence.
    #[must_use]
    pub fn fork(&mut self, label: &str) -> Self {
        Self::new(self.next_u64() ^ fnv1a_64(label))
    }

    /// Returns a number in inclusive range `min..=max`.
    pub fn range(&mut self, min: u32, max: u32) -> u32 {
        let span = u64::from(max - min) + 1;

        min + u32::try_from(self.next_u64() % span).expect("below the span")
    }

    /// Selects an item from `items`.
    ///
    /// # Panics
    ///
    /// Panics when `items` is empty.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.index(items.len())]
    }

    /// Removes a randomly selected item from `items`.
    ///
    /// # Panics
    ///
    /// Panics when `items` is empty.
    pub fn take<T>(&mut self, items: &mut Vec<T>) -> T {
        let at = self.index(items.len());

        items.remove(at)
    }

    /// Returns a random index below `len`.
    pub fn index(&mut self, len: usize) -> usize {
        let last = u32::try_from(len - 1).expect("a short list");

        self.range(0, last) as usize
    }

    /// Generates `len` characters, each picked from the ASCII `alphabet`.
    pub fn text(&mut self, alphabet: &[u8], len: u32) -> String {
        (0..len).map(|_| char::from(*self.pick(alphabet))).collect()
    }

    /// Generates `len` hexadecimal digits.
    pub fn hex(&mut self, len: u32) -> String {
        self.text(HEX_DIGITS, len)
    }

    /// Generates a MAC address of six colon-separated hexadecimal pairs.
    pub fn mac(&mut self) -> String {
        (0..6).map(|_| self.hex(2)).collect::<Vec<_>>().join(":")
    }

    /// Generates a nine-digit phone number as a digit from 1 to 9 followed by eight digits
    /// (`NpcInfo.NpcInfo`, which draws the nine digits in one number).
    pub fn phone(&mut self) -> String {
        format!("{}{}", self.range(1, 9), self.range(10_000_000, 99_999_999))
    }

    /// Generates a public IP outside every reserved range (`IP.GetRandomPublicIP`).
    pub fn public_ip(&mut self) -> String {
        loop {
            let address = format!(
                "{}.{}.{}.{}",
                self.range(1, 255),
                self.range(0, 255),
                self.range(0, 255),
                self.range(1, 254)
            );

            if !crate::is_group_reserved_ip(&address) {
                return address;
            }
        }
    }
}
