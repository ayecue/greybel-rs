//! Updates path-package versions in `Cargo.lock` and detects changes to path packages' external
//! dependencies between lockfile versions.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use anyhow::Result;
use cargo_lock::Lockfile;

use crate::language::LockChange;

/// `Cargo.lock`'s text.
pub struct CargoLock<'a> {
    text: &'a str,
}

impl<'a> CargoLock<'a> {
    pub const FILE_NAME: &str = "Cargo.lock";

    pub fn new(text: &'a str) -> Self {
        CargoLock { text }
    }

    /// Lock with the path package `name` at `version`, if it has it at `previous`.
    pub fn with_version(
        &self,
        name: &str,
        previous: &str,
        version: &str,
    ) -> Result<Option<String>> {
        let mut document: toml_edit::DocumentMut = self.text.parse()?;
        let Some(packages) = document
            .get_mut("package")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
        else {
            return Ok(None);
        };
        let is = |package: &toml_edit::Table| {
            package.get("name").and_then(toml_edit::Item::as_str) == Some(name)
                && package.get("version").and_then(toml_edit::Item::as_str) == Some(previous)
        };
        // Registry package of the same name and version makes `name previous` in dependency lists ambiguous.
        let external_same_version = packages
            .iter()
            .any(|package| is(package) && package.contains_key("source"));
        let mut found = false;

        for package in packages.iter_mut() {
            if is(package) && !package.contains_key("source") {
                package["version"] = toml_edit::value(version);
                found = true;
            }

            if external_same_version {
                continue;
            }

            if let Some(dependencies) = package
                .get_mut("dependencies")
                .and_then(toml_edit::Item::as_array_mut)
            {
                let old = format!("{name} {previous}");

                for dependency in dependencies.iter_mut() {
                    if dependency.as_str() == Some(old.as_str()) {
                        let decor = dependency.decor().clone();

                        *dependency = format!("{name} {version}").into();
                        *dependency.decor_mut() = decor;
                    }
                }
            }
        }

        Ok(found.then(|| document.to_string()))
    }

    /// Finds path packages with transitively changed external dependencies in the later lockfile `after`.
    pub fn changes_to(&self, after: &CargoLock<'_>) -> Result<Vec<LockChange>> {
        let before = LockGraph::parse(self.text)?;
        let after = LockGraph::parse(after.text)?;

        Ok(after
            .local
            .keys()
            .filter(|name| before.closure(name) != after.closure(name))
            .map(|name| LockChange::Package(name.clone()))
            .collect())
    }
}

/// Package of a lock file: its name, version and source.
type Key = (String, String, Option<String>);

/// `Cargo.lock`'s packages and what each depends on.
struct LockGraph {
    dependencies: BTreeMap<Key, Vec<Key>>,
    /// Maps path-package names without registry sources to keys.
    local: BTreeMap<String, Key>,
}

impl LockGraph {
    fn parse(text: &str) -> Result<Self> {
        let lockfile = Lockfile::from_str(text)?;
        let key = |name: &cargo_lock::Name,
                   version: &cargo_lock::Version,
                   source: &Option<cargo_lock::SourceId>| {
            (
                name.to_string(),
                version.to_string(),
                source.as_ref().map(ToString::to_string),
            )
        };
        let mut dependencies = BTreeMap::new();
        let mut local = BTreeMap::new();

        for package in &lockfile.packages {
            let package_key = key(&package.name, &package.version, &package.source);
            let edges = package
                .dependencies
                .iter()
                .map(|dependency| key(&dependency.name, &dependency.version, &dependency.source))
                .collect();

            if package.source.is_none() {
                local.insert(package.name.to_string(), package_key.clone());
            }

            dependencies.insert(package_key, edges);
        }

        Ok(LockGraph {
            dependencies,
            local,
        })
    }

    /// External packages `name` reaches, through path packages too.
    fn closure(&self, name: &str) -> BTreeSet<Key> {
        let mut visits = BTreeSet::new();
        let mut external = BTreeSet::new();
        let mut stack = self
            .local
            .get(name)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();

        while let Some(key) = stack.pop() {
            if !visits.insert(key.clone()) {
                continue;
            }

            if key.2.is_some() {
                external.insert(key.clone());
            }

            stack.extend(self.dependencies.get(&key).into_iter().flatten().cloned());
        }

        external
    }
}
