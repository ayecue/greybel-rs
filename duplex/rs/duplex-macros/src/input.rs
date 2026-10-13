use std::path::PathBuf;

use duplex_protocol::{MinorVersionTable, Role, VersionHistory};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, LitStr, Path, Token};

use crate::generate::Generator;

const MANIFEST_DIR: &str = "CARGO_MANIFEST_DIR";
const MAJOR: &str = "major";
const ROLE: &str = "role";
const TYPES: &str = "types";
const CORE: &str = "core";
const FROM: &str = "from";
const OLDER: &str = "older";
const NEWER: &str = "newer";

pub struct DefinitionFolder {
    path: LitStr,
}

#[derive(Default)]
struct Options {
    major: Option<u16>,
    role: Option<Role>,
    types: Option<Path>,
    core: Option<Path>,
    from: Option<u16>,
    older: Option<Path>,
    newer: Option<Path>,
}

pub struct TypesInput {
    pub folder: DefinitionFolder,
    pub major: u16,
    pub core: Option<Path>,
}

pub struct ProtocolInput {
    pub folder: DefinitionFolder,
    pub major: u16,
    pub role: Role,
    pub types: Option<Path>,
}

impl Options {
    fn parse(input: ParseStream<'_>, keywords: &[&str]) -> syn::Result<Self> {
        let mut options = Self::default();

        while input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            let key: Ident = input.parse()?;

            input.parse::<Token![=]>()?;

            let name = key.to_string();

            if !keywords.contains(&name.as_str()) {
                return Err(syn::Error::new(
                    key.span(),
                    format!("expected one of {}", keywords.join(", ")),
                ));
            }

            match name.as_str() {
                MAJOR => {
                    let major: LitInt = input.parse()?;

                    options.major = Some(major.base10_parse()?);
                }
                ROLE => {
                    let ident: Ident = input.parse()?;

                    options.role =
                        Some(Role::from_keyword(&ident.to_string()).ok_or_else(|| {
                            syn::Error::new(ident.span(), "the role is `client` or `provider`")
                        })?);
                }
                TYPES => options.types = Some(input.parse()?),
                FROM => {
                    let from: LitInt = input.parse()?;

                    options.from = Some(from.base10_parse()?);
                }
                OLDER => options.older = Some(input.call(Path::parse_mod_style)?),
                NEWER => options.newer = Some(input.call(Path::parse_mod_style)?),
                _ => options.core = Some(input.call(Path::parse_mod_style)?),
            }
        }

        Ok(options)
    }

    fn major(&self) -> syn::Result<u16> {
        self.major.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "the major generated is given, as `major = 1`",
            )
        })
    }
}

pub struct MigrationInput {
    pub folder: DefinitionFolder,
    pub from: u16,
    pub older: Path,
    pub newer: Path,
}

impl Parse for MigrationInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let folder = DefinitionFolder {
            path: input.parse()?,
        };
        let options = Options::parse(input, &[FROM, OLDER, NEWER])?;
        let required_error = |what: &str| {
            syn::Error::new(proc_macro2::Span::call_site(), format!("{what} is given"))
        };

        Ok(Self {
            folder,
            from: options
                .from
                .ok_or_else(|| required_error("the major migrated from, as `from = 1`,"))?,
            older: options
                .older
                .ok_or_else(|| required_error("the module of that major, as `older = v1`,"))?,
            newer: options.newer.ok_or_else(|| {
                required_error("the module of the major after it, as `newer = v2`,")
            })?,
        })
    }
}

impl Parse for TypesInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let folder = DefinitionFolder {
            path: input.parse()?,
        };
        let options = Options::parse(input, &[MAJOR, CORE])?;

        Ok(Self {
            folder,
            major: options.major()?,
            core: options.core,
        })
    }
}

impl Parse for ProtocolInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let folder = DefinitionFolder {
            path: input.parse()?,
        };
        let options = Options::parse(input, &[MAJOR, ROLE, TYPES])?;
        let major = options.major()?;
        let role = options.role.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "the role generated is given, as `role = client`",
            )
        })?;

        Ok(Self {
            folder,
            major,
            role,
            types: options.types,
        })
    }
}

impl DefinitionFolder {
    /// Resolves definition paths against `CARGO_MANIFEST_DIR`, or against the environment variable named by a leading
    /// `$NAME/` component.
    fn resolve(&self) -> syn::Result<PathBuf> {
        let path = self.path.value();
        let (variable, relative) = match path.strip_prefix('$') {
            Some(reference) => reference.split_once('/').unwrap_or((reference, "")),
            None => (MANIFEST_DIR, path.as_str()),
        };
        let base = std::env::var(variable).map_err(|_| {
            syn::Error::new(
                self.path.span(),
                format!("the environment variable `{variable}` is not set"),
            )
        })?;

        Ok(PathBuf::from(base).join(relative))
    }

    pub fn load(&self) -> syn::Result<(VersionHistory, Vec<PathBuf>)> {
        let folder = self.resolve()?;
        let versions = VersionHistory::load(&folder)
            .map_err(|error| syn::Error::new(self.path.span(), error.to_string()))?;
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .map_err(|error| syn::Error::new(self.path.span(), error.to_string()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .collect();

        files.sort();

        Ok((versions, files))
    }

    pub fn major<'v>(
        &self,
        versions: &'v VersionHistory,
        major: u16,
    ) -> syn::Result<MinorVersionTable<'v>> {
        versions.major(major).ok_or_else(|| {
            syn::Error::new(
                self.path.span(),
                format!("the folder has no versions of {major}.x"),
            )
        })
    }

    pub fn generate(
        &self,
        major: u16,
        generate: impl FnOnce(Generator<'_>) -> syn::Result<proc_macro2::TokenStream>,
    ) -> proc_macro::TokenStream {
        self.load()
            .and_then(|(versions, files)| {
                generate(Generator::new(&files, self.major(&versions, major)?))
            })
            .unwrap_or_else(syn::Error::into_compile_error)
            .into()
    }
}

#[cfg(test)]
#[path = "tests/input.rs"]
mod tests;
