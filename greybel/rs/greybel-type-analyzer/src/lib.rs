mod analysis;
mod analyzer;
mod annotation;
mod api;
mod database;
mod diagnostic;
mod ide;
mod index;
mod infer;
mod lookup;
mod model;
mod program;
mod types;

pub use analysis::{Analysis, Query, ReferenceTable};
pub use analyzer::Analyzer;
pub use annotation::{
    DocComment, FunctionDoc, FunctionTypeParameter, InvalidTag, ParamDoc, PropertyDoc, TemplateDoc,
    TextPosition, TypeDoc, TypeExpr, TypeLiteral,
};
pub use api::Api;
pub use database::{Analyze, Database, Db};
pub use diagnostic::{Diagnostic, DiagnosticKind, DiagnosticOptions, Severity};
pub use greybel_core::Dialect;
pub use greybel_documents::{Uri, Workspace};
pub use ide::{
    CallHierarchyCall, CallHierarchyItem, CodeAction, ColorInformation, CompletionItem,
    CompletionItemKind, DocumentSymbol, Fold, FoldKind, Hover, HoverKind, InlayHint, InlayHintKind,
    InlayHintOptions, Location, ParameterInformation, SignatureHelp, SymbolKind, TextEdit,
    WorkspaceEdit, WorkspaceSymbol,
};
pub use lookup::Member;
pub use model::{
    CustomType, DeclaredFunction, DeclaredId, DeclaredParameter, DocumentIndex, Documentation,
    FunctionData, FunctionId, GenericInstance, GenericInstanceId, Key, MapData, MapId, MapOrigin,
    NameId, Parameter, ScopeId, SemanticModel, Site, SlotId,
};
pub use types::{ApiFunction, FunctionRef, Type, TypeDisplay, TypeId, TypeTable};
