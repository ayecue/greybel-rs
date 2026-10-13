//! Per-VM compiled-code cache for launched binaries.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use greybel_interpreter::CodeId;
use greybel_source_map::SourceMap;

/// Newly compiled program or cached entry for a binary already added to the VM.
pub(super) enum Code {
    Program(greybel_interpreter::Program),
    Entry(CodeId),
}

/// VM-added binary code and its diagnostic source map.
/// Code compiled from the binary uses its own source map; code compiled from the binary's original source files uses an empty map.
#[derive(Clone)]
pub(crate) struct CachedCode {
    pub(crate) entry: CodeId,
    pub(crate) source_map: SourceMap,
}

/// Indexes VM-added code entries by [`launch_key`] hash.
#[derive(Default)]
pub(crate) struct LaunchCache(pub(crate) Rc<RefCell<HashMap<u64, CachedCode>>>);

/// Computes a [`LaunchCache`] binary key from `path` and `source`.
pub(crate) fn launch_key(path: &str, source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();

    (path, source).hash(&mut hasher);

    hasher.finish()
}
