use super::NodeList;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopeData {
    pub namespaces: NodeList,
    pub definitions: NodeList,
    pub returns: NodeList,
}

pub trait Scope {
    fn scope(&self) -> &ScopeData;
    fn scope_mut(&mut self) -> &mut ScopeData;
}
