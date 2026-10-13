mod format;
mod parts;
mod paths;

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use parts::RawUriComponentContainer;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::encoding::PercentEncoding;
use crate::error::{Result, UriError};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uri {
    components: Arc<UriComponentContainer>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UriComponentContainer {
    pub scheme: String,
    pub authority: String,
    pub path: String,
    pub query: String,
    pub fragment: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UriChange {
    pub scheme: Option<String>,
    pub authority: Option<String>,
    pub path: Option<String>,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl Uri {
    pub const FILE: &str = "file";
    pub const MEMORY: &str = "memory";

    pub fn parse(value: &str) -> Result<Self> {
        Self::parse_with(value, false)
    }

    pub fn parse_strict(value: &str) -> Result<Self> {
        Self::parse_with(value, true)
    }

    #[must_use]
    pub fn file(path: &str) -> Self {
        let mut path = if cfg!(windows) {
            path.replace('\\', "/")
        } else {
            path.to_owned()
        };
        let mut authority = String::new();

        if let Some(remainder) = path.strip_prefix("//") {
            match remainder.find('/') {
                None => {
                    authority = remainder.to_owned();
                    path = "/".to_owned();
                }
                Some(slash) => {
                    authority = remainder[..slash].to_owned();
                    path = remainder[slash..].to_owned();
                }
            }
        }

        Self::with_defaults(
            UriComponentContainer {
                scheme: Self::FILE.to_owned(),
                authority,
                path,
                ..UriComponentContainer::default()
            },
            false,
        )
    }

    #[must_use]
    pub fn memory(path: &str) -> Self {
        Self::with_defaults(
            UriComponentContainer {
                scheme: Self::MEMORY.to_owned(),
                path: path.to_owned(),
                ..UriComponentContainer::default()
            },
            false,
        )
    }

    #[must_use]
    pub fn label(&self) -> String {
        match self.components.scheme.as_str() {
            Self::FILE if self.components.authority.is_empty() => self.fs_path(),
            Self::MEMORY if self.components.authority.is_empty() => self.components.path.clone(),
            _ => self.to_string(),
        }
    }

    #[must_use]
    pub fn has_scheme(text: &str) -> bool {
        let Some(colon) = text.find(':') else {
            return false;
        };
        let scheme = &text[..colon];

        scheme.len() >= 2
            && scheme.starts_with(|character: char| character.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "+.-".contains(character))
    }

    #[must_use]
    pub fn is_drive_path(text: &str) -> bool {
        let bytes = text.as_bytes();

        bytes.len() >= 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes.get(2), None | Some(b'\\' | b'/'))
    }

    pub fn from_path_or_uri(text: &str) -> Result<Self> {
        if Self::has_scheme(text) {
            return Self::parse(text);
        }

        if Self::is_drive_path(text) {
            return Ok(Self::file(&text.replace('\\', "/")));
        }

        Ok(Self::file(text))
    }

    pub fn from_components(components: UriComponentContainer) -> Result<Self> {
        let uri = Self::new(components, false)?;

        uri.validate(true)?;

        Ok(uri)
    }

    pub fn with(&self, change: UriChange) -> Result<Self> {
        let components = UriComponentContainer {
            scheme: change
                .scheme
                .unwrap_or_else(|| self.components.scheme.clone()),
            authority: change
                .authority
                .unwrap_or_else(|| self.components.authority.clone()),
            path: change.path.unwrap_or_else(|| self.components.path.clone()),
            query: change
                .query
                .unwrap_or_else(|| self.components.query.clone()),
            fragment: change
                .fragment
                .unwrap_or_else(|| self.components.fragment.clone()),
        };

        if components == *self.components {
            return Ok(self.clone());
        }

        Self::new(components, false)
    }

    #[must_use]
    pub fn scheme(&self) -> &str {
        &self.components.scheme
    }

    #[must_use]
    pub fn authority(&self) -> &str {
        &self.components.authority
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.components.path
    }

    #[must_use]
    pub fn query(&self) -> &str {
        &self.components.query
    }

    #[must_use]
    pub fn fragment(&self) -> &str {
        &self.components.fragment
    }

    #[must_use]
    pub fn components(&self) -> &UriComponentContainer {
        &self.components
    }

    #[must_use]
    pub fn fs_path(&self) -> String {
        self.file_system_path()
    }

    #[must_use]
    pub fn to_unencoded_string(&self) -> String {
        self.format(true)
    }

    fn parse_with(value: &str, strict: bool) -> Result<Self> {
        let parts = RawUriComponentContainer::split(value);

        Self::new(
            UriComponentContainer {
                scheme: parts.scheme.to_owned(),
                authority: PercentEncoding::decode(parts.authority).into_owned(),
                path: PercentEncoding::decode(parts.path).into_owned(),
                query: PercentEncoding::decode(parts.query).into_owned(),
                fragment: PercentEncoding::decode(parts.fragment).into_owned(),
            },
            strict,
        )
    }

    fn new(components: UriComponentContainer, strict: bool) -> Result<Self> {
        let uri = Self::with_defaults(components, strict);

        uri.validate(strict)?;

        Ok(uri)
    }

    fn with_defaults(components: UriComponentContainer, strict: bool) -> Self {
        let scheme = if components.scheme.is_empty() && !strict {
            Self::FILE.to_owned()
        } else {
            components.scheme
        };
        let path = match scheme.as_str() {
            "https" | "http" | Self::FILE | Self::MEMORY if !components.path.starts_with('/') => {
                format!("/{}", components.path)
            }
            _ => components.path,
        };

        Self {
            components: Arc::new(UriComponentContainer {
                scheme,
                authority: components.authority,
                path,
                query: components.query,
                fragment: components.fragment,
            }),
        }
    }

    fn validate(&self, strict: bool) -> Result<()> {
        if self.components.scheme.is_empty() && strict {
            return Err(UriError::MissingScheme {
                authority: self.components.authority.clone(),
                path: self.components.path.clone(),
                query: self.components.query.clone(),
                fragment: self.components.fragment.clone(),
            });
        }

        if !self.components.scheme.is_empty() && !Self::is_scheme(&self.components.scheme) {
            return Err(UriError::IllegalScheme);
        }

        if !self.components.path.is_empty() {
            if !self.components.authority.is_empty() {
                if !self.components.path.starts_with('/') {
                    return Err(UriError::RelativePath);
                }
            } else if self.components.path.starts_with("//") {
                return Err(UriError::DoubleSlashPath);
            }
        }

        Ok(())
    }

    fn is_scheme(scheme: &str) -> bool {
        let is_word = |character: char| character.is_ascii_alphanumeric() || character == '_';
        let mut characters = scheme.chars();

        characters.next().is_some_and(is_word)
            && characters
                .all(|character| is_word(character) || matches!(character, '+' | '.' | '-'))
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.format(false))
    }
}

impl FromStr for Uri {
    type Err = UriError;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value)
    }
}

impl Serialize for Uri {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Uri {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;

        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
