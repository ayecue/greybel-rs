use greybel_core::{Ast, AstNode};

pub fn path_names(ast: &Ast, offset: usize) -> Vec<&str> {
    ast.path_at(u32::try_from(offset).expect("value fits the target integer type"))
        .into_iter()
        .map(|id| ast.kind(id).type_name())
        .collect()
}
