use gen_lsp_types::{self as lsp, LspNotificationMethod, Notification as LspNotification};

use crate::message::Notification;
use crate::server::{Server, State};

struct Route {
    method: LspNotificationMethod<'static>,
    handle: fn(&mut Server, &Notification),
}

macro_rules! route {
    ($notification:ty => $handler:ident) => {
        Route {
            method: <$notification>::METHOD,
            handle: |server, notification| {
                server.handle::<$notification>(notification, Server::$handler);
            },
        }
    };
}

impl Server {
    const NOTIFICATIONS: &[Route] = &[
        Route {
            method: lsp::InitializedNotification::METHOD,
            handle: |server, _| server.discover_workspace(),
        },
        route!(lsp::DidOpenTextDocumentNotification => did_open),
        route!(lsp::DidChangeTextDocumentNotification => did_change),
        route!(lsp::DidCloseTextDocumentNotification => did_close),
        route!(lsp::DidChangeConfigurationNotification => did_change_configuration),
        route!(lsp::DidChangeWatchedFilesNotification => did_change_watched_files),
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
