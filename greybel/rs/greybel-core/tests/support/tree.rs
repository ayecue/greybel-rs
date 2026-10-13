use greybel_core::{Ast, AstNode};

pub fn path_names(ast: &Ast, offset: usize) -> Vec<&str> {
    ast.path_at(offset as u32)
        .into_iter()
        .map(|id| ast.kind(id).type_name())
        .collect()
}
