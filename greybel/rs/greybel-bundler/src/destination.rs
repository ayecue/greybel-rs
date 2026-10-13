#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub directory: String,
    pub resources: String,
    pub temporary_folder_name: String,
}

impl Destination {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const NAME_LENGTH: usize = 5;

    #[must_use]
    pub fn new(directory: &str, rng: &mut fastrand::Rng) -> Self {
        let directory = directory.trim();
        let directory = directory.strip_suffix('/').unwrap_or(directory).to_owned();
        let resources = format!("{directory}/{}", Self::random_name(rng));

        Self {
            directory,
            resources,
            temporary_folder_name: Self::random_name(rng),
        }
    }

    fn random_name(rng: &mut fastrand::Rng) -> String {
        (0..Self::NAME_LENGTH)
            .map(|_| char::from(Self::ALPHABET[rng.usize(..Self::ALPHABET.len())]))
            .collect()
    }
}
