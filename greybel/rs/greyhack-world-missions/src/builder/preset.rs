use crate::config::{Modality, TemplateOption};

/// A heart option that a cross-information beat forces onto the next beat.
pub(super) struct Preset<'c> {
    pub(super) modality: Modality,
    pub(super) option: &'c TemplateOption,
}
