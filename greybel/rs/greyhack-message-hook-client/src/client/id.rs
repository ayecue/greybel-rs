use std::hash::{BuildHasher as _, Hasher as _, RandomState};
use std::sync::atomic::{AtomicU64, Ordering};

const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Default)]
pub(super) struct IdGenerator {
    keys: RandomState,
    counter: AtomicU64,
}

impl IdGenerator {
    pub(super) fn generate(&self) -> String {
        let mut pool = [0_u64; 2];

        for slot in &mut pool {
            let mut hasher = self.keys.build_hasher();

            hasher.write_u64(self.counter.fetch_add(1, Ordering::Relaxed));
            *slot = hasher.finish();
        }

        (0..16)
            .map(|i| {
                let slot = &mut pool[i / 8];
                let index = usize::try_from(*slot % 62).unwrap_or(0);

                *slot /= 62;

                char::from(ALPHABET[index])
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/id.rs"]
mod tests;
