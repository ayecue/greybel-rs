/// A `Cargo.lock` where `core` depends on serde at `version` and `left` on nothing.
pub fn lock_with_serde(version: &str) -> String {
    format!(
        "version = 4\n\n[[package]]\nname = \"core\"\nversion = \"0.1.0\"\ndependencies = [\n \"serde\",\n]\n\n\
         [[package]]\nname = \"left\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"serde\"\nversion = \"{version}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
    )
}
