//! Defaults and helpers for package repositories, source lists, and installation paths.

use serde::Deserialize;

/// Repository written to each new computer's `sources.txt`.
pub const DEFAULT_REPOSITORY: &str = "hackshop.org";
/// Port used by the default repository service.
pub const DEFAULT_REPOSITORY_PORT: u16 = 1542;
/// Path of the repository list.
pub const APT_SOURCES_PATH: &str = "/etc/apt/sources.txt";
/// Folder of the `aptclient` configuration.
pub const APT_FOLDER: &str = "/etc/apt";
/// Name of the repository cache file.
pub const APT_CACHE_NAME: &str = "aptcache.bin";
/// Name of the repository that serves the official packages, whatever the cache says
/// (`AptClientIntrinsics.show`).
pub const OFFICIAL_SERVER: &str = "official_server";
/// Libraries the official server does not offer (`AptClientIntrinsics.search`).
pub const OFFICIAL_OMISSIONS: &[&str] = &["metaxploit.so", "crypto.so", "librshell.so"];
/// Path of the repository cache.
pub const APT_CACHE_PATH: &str = "/etc/apt/aptcache.bin";
/// File size used for a plain binary installed by `aptclient`.
pub const APT_PACKAGE_SIZE: u64 = 360_448;
/// Default installation directory for library packages.
pub const APT_LIBRARY_FOLDER: &str = crate::LIBRARY_FOLDER;
/// Default installation directory for binary packages.
pub const APT_BINARY_FOLDER: &str = "/usr/bin";

/// Repository address and port used by `update` (`Service_repod.ClientConfigInfo.sourceList`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AptSource {
    pub address: String,
    pub port: u16,
}

/// Parses repositories from `sources.txt` in JSON dictionary order. Plain lines from older sandbox
/// worlds use the default port.
#[must_use]
pub fn parse_apt_sources(content: &str) -> Vec<AptSource> {
    if content.trim_start().starts_with('{') {
        return parse_source_list(content);
    }

    content
        .lines()
        .filter(|line| !line.is_empty())
        .map(|address| AptSource {
            address: address.to_owned(),
            port: DEFAULT_REPOSITORY_PORT,
        })
        .collect()
}

/// Formats `repositories` in order as `sources.txt` content (`Service_repod.ClientConfigInfo`).
#[must_use]
pub fn format_apt_sources(repositories: &[AptSource], official: bool) -> String {
    let entries = repositories
        .iter()
        .map(|source| {
            format!(
                "    {}: {}",
                serde_json::to_string(&source.address).expect("a repository address"),
                source.port
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    // `Service_repod.ClientConfigInfo` written with `JsonConvert.SerializeObject(config, Formatting.Indented)`: its fields
    // in declaration order, and an empty dictionary as `{}`. A new computer's file (`configGeneration["sources"]`) is
    // the same text as the default object written this way.
    let sources = if entries.is_empty() {
        String::from("{}")
    } else {
        format!("{{\n{entries}\n  }}")
    };

    format!("{{\n  \"official_server\": {official},\n  \"sourceList\": {sources}\n}}")
}

/// Checks whether `sources.txt` selects the game's official package server.
#[must_use]
pub fn parse_apt_official(content: &str) -> bool {
    #[derive(Deserialize)]
    struct Config {
        official_server: bool,
    }

    serde_json::from_str::<Config>(content).is_ok_and(|config| config.official_server)
}

/// Parses the repository dictionary in file order instead of sorting keys like `serde_json`'s default map.
fn parse_source_list(content: &str) -> Vec<AptSource> {
    let mut sources = Vec::new();
    let Some(open) = content
        .find("\"sourceList\"")
        .and_then(|key| content[key..].find('{').map(|open| key + open))
    else {
        return sources;
    };
    let mut remainder = &content[open + 1..];

    loop {
        remainder = remainder.trim_start();

        if remainder.starts_with('}') {
            break;
        }

        let Some((address, length)) = json_string_prefix(remainder) else {
            break;
        };

        remainder = remainder[length..].trim_start();

        let Some(after_colon) = remainder.strip_prefix(':') else {
            break;
        };

        remainder = after_colon.trim_start();

        let digits = remainder
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(remainder.len());
        let Ok(port) = remainder[..digits].parse::<u16>() else {
            break;
        };

        sources.push(AptSource { address, port });
        remainder = remainder[digits..].trim_start();
        if let Some(after_comma) = remainder.strip_prefix(',') {
            remainder = after_comma;
        }
    }

    sources
}

/// Parses the JSON string prefix of `input` and returns it with its byte length.
fn json_string_prefix(input: &str) -> Option<(String, usize)> {
    if !input.starts_with('"') {
        return None;
    }

    let mut escape = false;

    for (offset, character) in input.char_indices().skip(1) {
        if escape {
            escape = false;
            continue;
        }

        match character {
            '\\' => escape = true,
            '"' => {
                let length = offset + 1;

                return serde_json::from_str(&input[..length])
                    .ok()
                    .map(|value| (value, length));
            }
            _ => {}
        }
    }

    None
}

/// Returns `/lib` for libraries and `/usr/bin` for other binaries.
#[must_use]
pub const fn default_apt_install_folder(is_library: bool) -> &'static str {
    if is_library {
        APT_LIBRARY_FOLDER
    } else {
        APT_BINARY_FOLDER
    }
}
