use gen_lsp_types as lsp;
use greybel_type_analyzer::{
    CallHierarchyCall, CallHierarchyItem, DocumentSymbol, SymbolKind, WorkspaceSymbol,
};

use super::ToLsp;

impl ToLsp for DocumentSymbol {
    type Output = lsp::DocumentSymbol;

    fn to_lsp(&self) -> lsp::DocumentSymbol {
        lsp::DocumentSymbol::new(
            self.name.clone(),
            None,
            self.kind.to_lsp(),
            None,
            None,
            self.range.to_lsp(),
            self.selection_range.to_lsp(),
            Some(self.children.iter().map(ToLsp::to_lsp).collect()),
        )
    }
}

impl ToLsp for WorkspaceSymbol {
    type Output = lsp::WorkspaceSymbol;

    fn to_lsp(&self) -> lsp::WorkspaceSymbol {
        lsp::WorkspaceSymbol {
            location: lsp::WorkspaceSymbolLocation::Location(self.location.to_lsp()),
            data: None,
            base_symbol_information: lsp::BaseSymbolInformation {
                name: self.name.clone(),
                kind: self.kind.to_lsp(),
                tags: None,
                container_name: self.container.clone(),
            },
        }
    }
}

impl ToLsp for SymbolKind {
    type Output = lsp::SymbolKind;

    fn to_lsp(&self) -> lsp::SymbolKind {
        match self {
            SymbolKind::Variable => lsp::SymbolKind::Variable,
            SymbolKind::Function => lsp::SymbolKind::Function,
            SymbolKind::Property => lsp::SymbolKind::Property,
            SymbolKind::File => lsp::SymbolKind::File,
            SymbolKind::Type | SymbolKind::Class => lsp::SymbolKind::Class,
        }
    }
}

impl ToLsp for CallHierarchyItem {
    type Output = lsp::CallHierarchyItem;

    fn to_lsp(&self) -> lsp::CallHierarchyItem {
        lsp::CallHierarchyItem {
            name: self.name.clone(),
            kind: self.kind.to_lsp(),
            tags: None,
            detail: None,
            uri: self.uri.to_lsp(),
            range: self.range.to_lsp(),
            selection_range: self.selection_range.to_lsp(),
            data: None,
        }
    }
}

pub(crate) struct IncomingCall<'a>(pub &'a CallHierarchyCall);

impl ToLsp for IncomingCall<'_> {
    type Output = lsp::CallHierarchyIncomingCall;

    fn to_lsp(&self) -> lsp::CallHierarchyIncomingCall {
        lsp::CallHierarchyIncomingCall {
            from: self.0.item.to_lsp(),
            from_ranges: self.0.ranges.iter().map(ToLsp::to_lsp).collect(),
        }
    }
}

pub(crate) struct OutgoingCall<'a>(pub &'a CallHierarchyCall);

impl ToLsp for OutgoingCall<'_> {
    type Output = lsp::CallHierarchyOutgoingCall;

    fn to_lsp(&self) -> lsp::CallHierarchyOutgoingCall {
        lsp::CallHierarchyOutgoingCall {
            to: self.0.item.to_lsp(),
            from_ranges: self.0.ranges.iter().map(ToLsp::to_lsp).collect(),
        }
    }
}
