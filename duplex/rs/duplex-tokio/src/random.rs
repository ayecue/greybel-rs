/// Returns 16 non-cryptographic random bytes for connection IDs, not cryptographic keys.
#[must_use]
pub fn random_bytes() -> [u8; 16] {
    std::array::from_fn(|_| fastrand::u8(..))
}
