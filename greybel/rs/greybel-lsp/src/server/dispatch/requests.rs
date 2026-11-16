use gen_lsp_types::{self as lsp, ErrorCodes, LspRequestMethod, Request as LspRequest};

use crate::client_requests::Project;
use crate::error::{ResponseError, Result};
use crate::message::{Request, Response};
use crate::server::{Server, State};

struct Route {
    method: LspRequestMethod<'static>,
    handle: fn(&mut Server, Request) -> Response,
}

macro_rules! route {
    ($request:ty => $handler:ident) => {
        Route {
            method: <$request>::METHOD,
            handle: |server, request| server.reply::<$request>(request, Server::$handler),
        }
    };
}

impl Server {
    const REQUESTS: &[Route] = &[
        route!(lsp::InitializeRequest => initialize),
        Route {
            method: lsp::ShutdownRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::ShutdownRequest>(request, |server, ()| {
                    server.shutdown();

                    Ok(())
                })
            },
        },
        route!(lsp::HoverRequest => hover),
        route!(lsp::CompletionRequest => completion),
        route!(lsp::CompletionResolveRequest => completion_resolve),
        route!(lsp::SignatureHelpRequest => signature_help),
        route!(lsp::DefinitionRequest => definition),
        route!(lsp::TypeDefinitionRequest => type_definition),
        route!(lsp::CallHierarchyPrepareRequest => prepare_call_hierarchy),
        route!(lsp::CallHierarchyIncomingCallsRequest => incoming_calls),
        route!(lsp::CallHierarchyOutgoingCallsRequest => outgoing_calls),
        route!(lsp::ReferencesRequest => references),
        route!(lsp::DocumentHighlightRequest => document_highlight),
        route!(lsp::PrepareRenameRequest => prepare_rename),
        route!(lsp::RenameRequest => rename),
        route!(lsp::SemanticTokensRequest => semantic_tokens_full),
        route!(lsp::DocumentSymbolRequest => document_symbol),
        route!(lsp::WorkspaceSymbolRequest => workspace_symbol),
        route!(lsp::CodeActionRequest => code_action),
        route!(lsp::FoldingRangeRequest => folds),
        route!(lsp::InlayHintRequest => inlay_hints),
        route!(lsp::SelectionRangeRequest => selection_range),
        route!(lsp::DocumentColorRequest => document_colors),
        route!(lsp::ColorPresentationRequest => color_presentations),
        route!(lsp::DocumentFormattingRequest => format_document),
    ];

    /// Returns `request`'s response, or `None` if the server will respond later.
    pub(in crate::server) fn request(&mut self, request: Request) -> Option<Response> {
        if let Some(refusal) = self.refusal(&request) {
            return Some(Response::error(Some(request.id), refusal));
        }

        if request.method == Project::METHOD.as_str() {
            return self.project(request);
        }

        Some(
            match Self::REQUESTS
                .iter()
                .find(|route| route.method.as_str() == request.method)
            {
                Some(route) => (route.handle)(self, request),
                None => {
                    let message = format!("\"{}\" is not supported", request.method);

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
