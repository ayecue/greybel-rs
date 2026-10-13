use std::collections::BTreeMap;

use greybel_documents::Uri;
use greybel_runtime::protocol::{Environment, Launch};

use crate::project::Project;

/// Execution environment for a project run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Simulated world generated from the project seed.
    Simulation,
    /// Grey Hack single-player through the message hook at `port`, using the project's program name.
    Game { port: u16 },
}

impl Project {
    /// Project run of `entry` in `place` with `variables` for `#envar`; `debug` enables breakpoints and stepping.
    #[must_use]
    pub fn launch(
        &self,
        entry: &Uri,
        place: Place,
        variables: BTreeMap<String, String>,
        debug: bool,
    ) -> Launch<'static> {
        let run = &self.manifest.run;
        let environment = match place {
            Place::Simulation => Environment::Mock {
                seed: run.seed.clone(),
                memory_limit: run.memory_limit,
            },
            Place::Game { port } => Environment::Game { port },
        };

        Launch {
            entry: entry.to_string().into(),
            root: self.root.as_ref().map(|root| root.to_string().into()),
            extensions: self.manifest.extensions.clone().into(),
            environment,
            debug,
            stop_on_entry: false,
            params: run.params.clone().into(),
            variables,
            strict_mode: self.manifest.strict_mode,
            program_name: run.program_name.clone().into(),
        }
    }
}
