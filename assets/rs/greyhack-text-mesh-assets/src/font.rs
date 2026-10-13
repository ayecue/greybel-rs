use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFile {
    pub family: &'static str,
    pub file_name: &'static str,
    pub data: &'static [u8],
    pub license: &'static str,
    pub assets: &'static [&'static str],
}

macro_rules! font {
    ($family:literal, $file:literal, $license:literal, [$($asset:literal),* $(,)?]) => {
        FontFile {
            family: $family,
            file_name: $file,
            data: crate::bundle::file(concat!("fonts/", $file)),
            license: include_str!(concat!("../licenses/", $license)),
            assets: &[$($asset),*],
        }
    };
}

impl FontFile {
    /// Returns the source fonts and SIL OFL licenses for Grey Hack's font assets.
    #[must_use]
    pub fn all() -> &'static [Self] {
        static ALL: LazyLock<Vec<FontFile>> = LazyLock::new(|| {
            vec![
                font!(
                    "Source Code Pro",
                    "SourceCodePro-Regular.ttf",
                    "SourceCodePro-LICENSE.md",
                    [
                        "SourceCodePro-Regular SDF",
                        "SourceCodePro-Regular SDF_turco",
                        "SourceCodePro-Regular-PL SDF",
                        "SourceCodePro-Regular-SDF_draw",
                    ]
                ),
                font!(
                    "Liberation Sans",
                    "LiberationSans-Regular.ttf",
                    "LiberationSans-LICENSE.txt",
                    ["LiberationSans SDF", "LiberationSans SDF_turco"]
                ),
                font!(
                    "JetBrains Mono",
                    "JetBrainsMono-Regular.ttf",
                    "JetBrainsMono-LICENSE.txt",
                    ["JetBrainsMono-Regular SDF_RUUK"]
                ),
                font!(
                    "M PLUS 1 Code",
                    "MPLUS1Code-Regular.ttf",
                    "MPLUS1Code-LICENSE.txt",
                    ["Mplus1Code-Regular SDF_JP"]
                ),
                font!(
                    "Taviraj",
                    "Taviraj-Medium.ttf",
                    "Taviraj-LICENSE.txt",
                    ["Taviraj-Medium SDF"]
                ),
            ]
        });

        &ALL
    }

    #[must_use]
    pub fn find(asset_name: &str) -> Option<&'static Self> {
        Self::all().iter().find(|font| {
            font.assets
                .iter()
                .any(|name| name.eq_ignore_ascii_case(asset_name))
        })
    }
}
