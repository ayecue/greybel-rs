use crate::config::{Modality, TemplateOption};

/// `ProcMissionBuilder.BeatSelection` without its runtime tags.
#[derive(Clone)]
pub(super) struct Beat<'c> {
    pub(super) index: i32,
    pub(super) network: i32,
    pub(super) local: i32,
    pub(super) modality: Modality,
    pub(super) option: &'c TemplateOption,
}
