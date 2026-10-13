use gen_lsp_types as lsp;
use greybel_type_analyzer::{DocumentSymbol, SymbolKind, WorkspaceSymbol};

use super::{location, range};

#[must_use]
pub fn document_symbol(symbol: &DocumentSymbol) -> lsp::DocumentSymbol {
    lsp::DocumentSymbol::new(
        symbol.name.clone(),
        None,
        symbol_kind(symbol.kind),
        None,
        None,
        range(symbol.range),
        range(symbol.selection_range),
        Some(symbol.children.iter().map(document_symbol).collect()),
    )
}

#[must_use]
pub fn workspace_symbol(symbol: &WorkspaceSymbol) -> lsp::WorkspaceSymbol {
    lsp::WorkspaceSymbol {
        location: lsp::WorkspaceSymbolLocation::Location(location(&symbol.location)),
        data: None,
        base_symbol_information: lsp::BaseSymbolInformation {
            name: symbol.name.clone(),
            kind: symbol_kind(symbol.kind),
            tags: None,
            container_name: symbol.container.clone(),
        },
    }
}

fn symbol_kind(kind: SymbolKind) -> lsp::SymbolKind {
    match kind {
        SymbolKind::Variable => lsp::SymbolKind::Variable,
        SymbolKind::Function => lsp::SymbolKind::Function,
        SymbolKind::Property => lsp::SymbolKind::Property,
    }
}
