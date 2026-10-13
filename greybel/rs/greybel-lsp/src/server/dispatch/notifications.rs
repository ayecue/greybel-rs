use gen_lsp_types::{self as lsp, LspNotificationMethod, Notification as LspNotification};

use crate::message::Notification;
use crate::server::{Server, State};

struct Route {
    method: LspNotificationMethod<'static>,
    handle: fn(&mut Server, &Notification),
}

impl Server {
    const NOTIFICATIONS: &[Route] = &[
        Route {
            method: lsp::InitializedNotification::METHOD,
            handle: |server, _| server.discover_workspace(),
        },
        Route {
            method: lsp::DidOpenTextDocumentNotification::METHOD,
            handle: |server, notification| {
                server.handle::<lsp::DidOpenTextDocumentNotification>(notification, Self::did_open);
            },
        },
        Route {
            method: lsp::DidChangeTextDocumentNotification::METHOD,
            handle: |server, notification| {
                server.handle::<lsp::DidChangeTextDocumentNotification>(
                    notification,
                    Self::did_change,
                );
            },
        },
        Route {
            method: lsp::DidCloseTextDocumentNotification::METHOD,
            handle: |server, notification| {
                server
                    .handle::<lsp::DidCloseTextDocumentNotification>(notification, Self::did_close);
            },
        },
        Route {
            method: lsp::DidChangeConfigurationNotification::METHOD,
            handle: |server, notification| {
                server.handle::<lsp::DidChangeConfigurationNotification>(
                    notification,
                    Self::did_change_configuration,
                );
            },
        },
        Route {
            method: lsp::DidChangeWatchedFilesNotification::METHOD,
            handle: |server, notification| {
                server.handle::<lsp::DidChangeWatchedFilesNotification>(
                    notification,
                    Self::did_change_watched_files,
                );
            },
        },
    ];

    pub(in crate::server) fn notification(&mut self, notification: &Notification) {
        if notification.is::<lsp::ExitNotification>() {
            self.exit();

            return;
        }

        if self.state != State::Active {
            return;
        }

        if let Some(route) = Self::NOTIFICATIONS
            .iter()
            .find(|route| route.method.as_str() == notification.method)
        {
            (route.handle)(self, notification);
        }
    }

    fn handle<N: LspNotification>(
        &mut self,
        notification: &Notification,
        handle: impl FnOnce(&mut Self, N::Params),
    ) {
        if let Ok(params) = notification.params::<N>() {
            handle(self, params);
        }
    }
}
