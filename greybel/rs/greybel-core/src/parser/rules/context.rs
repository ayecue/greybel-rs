#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExprContext {
    pub as_lval: bool,
    pub statement_start: bool,
}
