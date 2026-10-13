use gen_lsp_types::{self as lsp, ErrorCodes, LspRequestMethod, Request as LspRequest};

use crate::error::{ResponseError, Result};
use crate::files::ProjectRequest;
use crate::message::{Request, Response};
use crate::server::{Server, State};

struct Route {
    method: LspRequestMethod<'static>,
    handle: fn(&mut Server, Request) -> Response,
}

impl Server {
    const REQUESTS: &[Route] = &[
        Route {
            method: lsp::InitializeRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::InitializeRequest>(request, Self::initialize)
            },
        },
        Route {
            method: lsp::ShutdownRequest::METHOD,
            handle: |server, request| server.reply::<lsp::ShutdownRequest>(request, Self::shutdown),
        },
        Route {
            method: lsp::HoverRequest::METHOD,
            handle: |server, request| server.reply::<lsp::HoverRequest>(request, Self::hover),
        },
        Route {
            method: lsp::CompletionRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::CompletionRequest>(request, Self::completion)
            },
        },
        Route {
            method: lsp::CompletionResolveRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::CompletionResolveRequest>(request, Self::completion_resolve)
            },
        },
        Route {
            method: lsp::SignatureHelpRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::SignatureHelpRequest>(request, Self::signature_help)
            },
        },
        Route {
            method: lsp::DefinitionRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DefinitionRequest>(request, Self::definition)
            },
        },
        Route {
            method: lsp::ReferencesRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::ReferencesRequest>(request, Self::references)
            },
        },
        Route {
            method: lsp::DocumentHighlightRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DocumentHighlightRequest>(request, Self::document_highlight)
            },
        },
        Route {
            method: lsp::PrepareRenameRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::PrepareRenameRequest>(request, Self::prepare_rename)
            },
        },
        Route {
            method: lsp::RenameRequest::METHOD,
            handle: |server, request| server.reply::<lsp::RenameRequest>(request, Self::rename),
        },
        Route {
            method: lsp::SemanticTokensRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::SemanticTokensRequest>(request, Self::semantic_tokens_full)
            },
        },
        Route {
            method: lsp::DocumentSymbolRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DocumentSymbolRequest>(request, Self::document_symbol)
            },
        },
        Route {
            method: lsp::WorkspaceSymbolRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::WorkspaceSymbolRequest>(request, Self::workspace_symbol)
            },
        },
        Route {
            method: lsp::CodeActionRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::CodeActionRequest>(request, Self::code_action)
            },
        },
        Route {
            method: lsp::DocumentFormattingRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DocumentFormattingRequest>(request, Self::format_document)
            },
        },
    ];

    /// Returns `request`'s response, or `None` if it will be answered later.
    pub(in crate::server) fn request(&mut self, request: Request) -> Option<Response> {
        if let Some(refusal) = self.refusal(&request) {
            return Some(Response::error(Some(request.id), refusal));
        }

        if request.method == ProjectRequest::METHOD.as_str() {
            return self.project(request);
        }

        Some(
            match Self::REQUESTS
                .iter()
                .find(|route| route.method.as_str() == request.method)
            {
                Some(route) => (route.handle)(self, request),
                None => {
                    let message = format!("{} is not supported", request.method);

                    Response::error(
                        Some(request.id),
                        ResponseError::new(ErrorCodes::MethodNotFound, message),
                    )
                }
            },
        )
    }

    fn refusal(&self, request: &Request) -> Option<ResponseError> {
        let initialize = request.is::<lsp::InitializeRequest>();
        let (code, message) = match self.state {
            State::Initial if !initialize => (
                ErrorCodes::ServerNotInitialized,
                "the server is not initialized",
            ),
            State::Active if initialize => (
                ErrorCodes::InvalidRequest,
                "the server is already initialized",
            ),
            State::Teardown | State::Exit(_) => {
                (ErrorCodes::InvalidRequest, "the server is closed")
            }
            _ => return None,
        };

        Some(ResponseError::new(code, message))
    }

    fn reply<R: LspRequest>(
        &mut self,
        request: Request,
        handle: impl FnOnce(&mut Self, R::Params) -> Result<R::Result>,
    ) -> Response {
        let params = match request.params::<R>() {
            Ok(params) => params,
            Err(error) => {
                return Response::error(
                    Some(request.id),
                    ResponseError::invalid_params(error.to_string()),
                );
            }
        };

        match handle(self, params) {
            Ok(result) => Response::ok::<R>(request.id, result),
            Err(error) => Response::error(Some(request.id), error),
        }
    }
}
