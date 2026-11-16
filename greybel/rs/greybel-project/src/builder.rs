use std::collections::BTreeMap;

use greybel_builder::protocol::{AutoCompile, Build, TranspileMode, TranspileSettings};
use greybel_documents::Uri;

use crate::manifest::{BuildMode, Indentation};
use crate::project::Project;

impl Project {
    /// Project build transpilation settings and `variables` for `#envar`.
    #[must_use]
    pub fn transpile_settings(&self, variables: BTreeMap<String, String>) -> TranspileSettings {
        let manifest = &self.manifest;
        let build = &manifest.build;
        let mode = match build.mode {
            BuildMode::Default => TranspileMode::Default,
            BuildMode::Uglify => TranspileMode::Uglify {
                disable_literals_optimization: build.disable_literals_optimization,
                disable_namespaces_optimization: build.disable_namespaces_optimization,
            },
            BuildMode::Beautify => TranspileMode::Beautify {
                retain_parentheses: manifest.format.retain_parentheses,
                indentation_spaces: (manifest.format.indentation == Indentation::Whitespace)
                    .then_some(manifest.format.indentation_spaces),
            },
        };

        TranspileSettings {
            mode,
            obfuscation: build.obfuscation,
            strict_mode: manifest.strict_mode,
            variables,
            excluded_names: build.excluded_namespaces.clone(),
            disabled_syntax: manifest.syntax.off_names(),
        }
    }

    /// Project build of `entries` with `variables` for `#envar`. If `deploy_port` is set, deploys to the game through
    /// that message-hook port.
    #[must_use]
    pub fn build(
        &self,
        entries: &[Uri],
        variables: BTreeMap<String, String>,
        deploy_port: Option<u16>,
    ) -> Build<'static> {
        let build = &self.manifest.build;
        let allow_import = if deploy_port.is_some() {
            build.deploy.allow_import
        } else {
            build.installer.allow_import
        };

        Build {
            entries: entries
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .into(),
            root: self.root.as_ref().map(|root| root.to_string().into()),
            extensions: self.manifest.extensions.clone().into(),
            transpile: self.transpile_settings(variables),
            ingame_directory: build.ingame_directory.clone().into(),
            output_name: build.output_name.clone().map(Into::into),
            auto_compile: build.auto_compile.then_some(AutoCompile { allow_import }),
            installer_length: build.installer.active.then_some(build.installer.max_chars),
            deploy_port,
        }
    }
}
