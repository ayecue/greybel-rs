mod dispatch;
mod documents;
mod features;
mod project;
mod session;
mod state;

use gen_lsp_types as lsp;
use greybel_core::Dialect;
use greybel_documents::{Uri, Workspace};
#[cfg(not(target_family = "wasm"))]
use greybel_documents_fs_provider::FsProvider;
use greybel_type_analyzer::{Analyzer, Database};
use rustc_hash::FxHashMap;

use self::documents::{ClientFileTable, DocumentTable};
use self::project::ProjectTable;
use self::session::ClientSupport;
use self::state::{FetchHandle, FileAccess, Outbound};
use crate::message::Message;
use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    Active,
    Teardown,
    Exit(i32),
}

pub struct Server {
    state: State,
    settings: Settings,
    client: ClientSupport,
    folders: Vec<Uri>,
    access: FileAccess,
    dialect: Dialect,
    workspace: Workspace<Database>,
    documents: DocumentTable,
    files: ClientFileTable,
    fetch_handle: FetchHandle,
    projects: ProjectTable,
    outbound: Outbound,
    published_diagnostics: FxHashMap<Uri, Vec<lsp::Diagnostic>>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub const NAME: &str = "greybel-lsp";

    #[must_use]
    pub fn new() -> Self {
        let settings = Settings::default();
        let dialect = Dialect::GreyScript;

        Self {
            state: State::Initial,
            access: FileAccess::Client(settings.resolution(&[])),
            workspace: Workspace::with_database(
                Database::default(),
                settings.document_options(dialect),
            ),
            dialect,
            settings,
            client: ClientSupport::default(),
            folders: Vec::new(),
            documents: DocumentTable::default(),
            files: ClientFileTable::default(),
            fetch_handle: FetchHandle::default(),
            projects: ProjectTable::default(),
            outbound: Outbound::default(),
            published_diagnostics: FxHashMap::default(),
        }
    }

    #[cfg(not(target_family = "wasm"))]
    #[must_use]
    pub fn with_local_files() -> Self {
        let mut server = Self::new();

        server.access = FileAccess::Local(FsProvider::new(server.access.resolution().clone()));

        server
    }

    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        match self.state {
            State::Exit(code) => Some(code),
            _ => None,
        }
    }

    fn analyzer(&self) -> Analyzer<'_> {
        Analyzer::new(&self.workspace)
    }

    pub fn receive(&mut self, message: Message) -> Vec<Message> {
        match message {
            Message::Request(request) => {
                if let Some(response) = self.request(request) {
                    self.outbound.push(Message::Response(response));
                }
            }
            Message::Notification(notification) => self.notification(&notification),
            Message::Response(response) => self.response(&response),
        }

        if self.state == State::Active {
            self.publish_diagnostics();
        }

        self.outbound.take()
    }
}
