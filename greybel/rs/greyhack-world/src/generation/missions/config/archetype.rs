use serde::Deserialize;

use super::closure::ClosureConfig;
use super::heart::HeartConfig;
use super::modality::ModalityConfig;
use super::template_option::TemplateOption;
use super::verb::VerbConfig;
use super::{Archetype, Modality, Source, Verb};

/// `MissionConfig.ArchetypeConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ArchetypeConfig {
    pub archetype: Archetype,
    pub source: Source,
    pub groups: Vec<String>,
    pub starters: Vec<TemplateOption>,
    pub heart: HeartConfig,
    pub closure: ClosureConfig,
}

impl ArchetypeConfig {
    /// `ProcMissionsDefaults.EnsureArchetypeShape`: every verb has every modality, and group names are trimmed.
    pub(super) fn ensure_shape(&mut self) {
        for group in &mut self.groups {
            *group = group.trim().to_owned();
        }

        for starter in &mut self.starters {
            starter.ensure_shape();
        }

        for entry in &mut self.closure.entries {
            entry.option.ensure_shape();
        }

        for verb in Verb::ALL {
            let index = match self
                .heart
                .verbs
                .iter()
                .position(|config| config.verb == *verb)
            {
                Some(index) => index,
                None => {
                    self.heart.verbs.push(VerbConfig {
                        verb: *verb,
                        modalities: Vec::new(),
                    });

                    self.heart.verbs.len() - 1
                }
            };
            let config = &mut self.heart.verbs[index];

            for modality in Modality::ALL {
                let at = match config
                    .modalities
                    .iter()
                    .position(|config| config.modality == *modality)
                {
                    Some(at) => at,
                    None => {
                        config.modalities.push(ModalityConfig {
                            modality: *modality,
                            options: Vec::new(),
                        });

                        config.modalities.len() - 1
                    }
                };

                for option in &mut config.modalities[at].options {
                    option.ensure_shape();
                }
            }
        }
    }
}
