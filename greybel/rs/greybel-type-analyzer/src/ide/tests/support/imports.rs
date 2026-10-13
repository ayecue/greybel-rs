use greybel_core::Dialect;
use greybel_documents::ImportKind;

use super::super::ImportTarget;

/// Returns the import kind and prefix for the target ending at `source`'s end, or `None` if there is no target.
pub fn at(source: &str, dialect: Dialect) -> Option<(ImportKind, &str)> {
    ImportTarget::at(source, source.len(), dialect).map(|target| (target.kind, target.prefix))
}
