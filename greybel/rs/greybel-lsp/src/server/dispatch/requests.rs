use gen_lsp_types::{self as lsp, ErrorCodes, LspRequestMethod, Request as LspRequest};

use crate::error::{ResponseError, Result};
use crate::files::Project;
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
                server.reply::<lsp::InitializeRequest>(request, |server, params| {
                    Self::initialize(server, &params)
                })
            },
        },
        Route {
            method: lsp::ShutdownRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::ShutdownRequest>(request, |server, params| {
                    server.shutdown(params);

                    Ok(())
                })
            },
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
            method: lsp::TypeDefinitionRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::TypeDefinitionRequest>(request, Self::type_definition)
            },
        },
        Route {
            method: lsp::CallHierarchyPrepareRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::CallHierarchyPrepareRequest>(
                    request,
                    Self::prepare_call_hierarchy,
                )
            },
        },
        Route {
            method: lsp::CallHierarchyIncomingCallsRequest::METHOD,
            handle: |server, request| {
                server
                    .reply::<lsp::CallHierarchyIncomingCallsRequest>(request, Self::incoming_calls)
            },
        },
        Route {
            method: lsp::CallHierarchyOutgoingCallsRequest::METHOD,
            handle: |server, request| {
                server
                    .reply::<lsp::CallHierarchyOutgoingCallsRequest>(request, Self::outgoing_calls)
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
                server.reply::<lsp::SemanticTokensRequest>(request, |server, params| {
                    Self::semantic_tokens_full(server, &params)
                })
            },
        },
        Route {
            method: lsp::DocumentSymbolRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DocumentSymbolRequest>(request, |server, params| {
                    Self::document_symbol(server, &params)
                })
            },
        },
        Route {
            method: lsp::WorkspaceSymbolRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::WorkspaceSymbolRequest>(request, |server, params| {
                    Ok(Some(server.workspace_symbol(&params)))
                })
            },
        },
        Route {
            method: lsp::CodeActionRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::CodeActionRequest>(request, |server, params| {
                    Self::code_action(server, &params)
                })
            },
        },
        Route {
            method: lsp::FoldingRangeRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::FoldingRangeRequest>(request, |server, params| {
                    Self::folds(server, &params)
                })
            },
        },
        Route {
            method: lsp::InlayHintRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::InlayHintRequest>(request, |server, params| {
                    Self::inlay_hints(server, &params)
                })
            },
        },
        Route {
            method: lsp::SelectionRangeRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::SelectionRangeRequest>(request, |server, params| {
                    Self::selection_range(server, &params)
                })
            },
        },
        Route {
            method: lsp::DocumentColorRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DocumentColorRequest>(request, |server, params| {
                    Self::document_colors(server, &params)
                })
            },
        },
        Route {
            method: lsp::ColorPresentationRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::ColorPresentationRequest>(request, |_, params| {
                    Ok(Self::color_presentations(&params))
                })
            },
        },
        Route {
            method: lsp::DocumentFormattingRequest::METHOD,
            handle: |server, request| {
                server.reply::<lsp::DocumentFormattingRequest>(request, |server, params| {
                    Self::format_document(server, &params)
                })
            },
        },
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
