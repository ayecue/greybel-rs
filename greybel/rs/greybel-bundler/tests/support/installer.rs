use greybel_bundler::{BuildOutput, Installer, InstallerScript};

/// Generates installer scripts for `output` using [`Installer::DEFAULT_MAX_CHARS`], panicking if they do not fit.
pub fn scripts(output: &BuildOutput) -> Vec<InstallerScript> {
    Installer::new(output, Installer::DEFAULT_MAX_CHARS)
        .scripts()
        .expect("fits")
}

pub fn char_count(text: &str) -> usize {
    text.encode_utf16().count()
}
