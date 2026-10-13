use gen_lsp_types as lsp;
use serde::Deserialize;
use serde_json::Value;

use super::Server;
use super::state::Expectation;
use crate::settings::Settings;

impl Server {
    pub(super) fn request_configuration(&mut self) {
        self.outbound.request::<lsp::ConfigurationRequest>(
            lsp::ConfigurationParams {
                items: vec![lsp::ConfigurationItem {
                    scope_uri: None,
                    section: Some(Settings::SECTION.to_owned()),
                }],
            },
            Expectation::Configuration,
        );
    }

    pub(super) fn apply_settings(&mut self, value: Option<Value>) -> bool {
        let settings = value
            .and_then(|value| Settings::deserialize(&value).ok())
            .unwrap_or_default();
        let rescan = settings.file_extensions != self.settings.file_extensions
            || settings.type_analyzer != self.settings.type_analyzer;

        self.settings = settings;
        self.access
            .set_resolution(self.settings.resolution(&self.folders));
        self.workspace
            .set_options(self.settings.document_options(self.dialect));

        rescan
    }

    pub(super) fn did_change_configuration(&mut self, params: lsp::DidChangeConfigurationParams) {
        let section = params.settings.get(Settings::SECTION).cloned();

        match section {
            Some(section) => {
                if self.apply_settings(Some(section)) {
                    self.find_files();
                }
            }
            None if self.client.configuration => self.request_configuration(),
            None => {}
        }
    }
}
