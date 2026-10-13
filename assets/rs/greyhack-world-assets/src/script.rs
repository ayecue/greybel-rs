use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Script {
    pub command: &'static str,
    pub source: &'static str,
}

impl Script {
    /// Returns command scripts sorted by command name.
    #[must_use]
    pub fn all() -> &'static [Self] {
        static ALL: LazyLock<Vec<Script>> = LazyLock::new(|| {
            let mut scripts: Vec<Script> = crate::bundle::BUNDLE
                .names()
                .filter_map(|name| {
                    let command = name.strip_prefix("scripts/")?.strip_suffix(".src")?;

                    Some(Script {
                        command,
                        source: crate::bundle::text(name),
                    })
                })
                .collect();

            scripts.sort_by_key(|script| script.command);

            scripts
        });

        &ALL
    }

    #[must_use]
    pub fn find(command: &str) -> Option<&'static Self> {
        Self::all().iter().find(|script| script.command == command)
    }
}
