use crate::analysis::Analysis;
use crate::lookup::{Lookup, Member};
use crate::model::{Documentation, KeyRef};
use crate::types::{TypeDisplay, TypeId, TypeTable};

pub struct Query<'a> {
    analysis: &'a Analysis,
    types: TypeTable,
}

impl<'a> Query<'a> {
    #[must_use]
    pub fn new(analysis: &'a Analysis) -> Self {
        Self {
            analysis,
            types: analysis.types().clone(),
        }
    }

    #[must_use]
    pub fn analysis(&self) -> &'a Analysis {
        self.analysis
    }

    #[must_use]
    pub fn types(&self) -> &TypeTable {
        &self.types
    }

    pub fn member_type(&mut self, ty: TypeId, name: &str) -> TypeId {
        self.lookup().member_type(ty, KeyRef::String(name))
    }

    pub fn members(&mut self, ty: TypeId) -> Vec<Member> {
        self.lookup().members(ty)
    }

    pub fn invocation_type(&mut self, ty: TypeId) -> TypeId {
        self.lookup().invocation_type(ty)
    }

    pub fn general_function_type(&mut self, name: &str) -> Option<TypeId> {
        self.lookup().general_function_type(name)
    }

    #[must_use]
    pub fn documentation(&self, ty: TypeId) -> Option<Documentation> {
        self.analysis.documentation_in(&self.types, ty)
    }

    #[must_use]
    pub fn is_deprecated(&self, ty: TypeId) -> bool {
        self.analysis.model().deprecation(&self.types, ty).is_some()
    }

    #[must_use]
    pub fn display(&self, ty: TypeId) -> TypeDisplay<'_> {
        self.analysis.notation_in(&self.types).display(ty)
    }

    fn lookup(&mut self) -> Lookup<'_> {
        Lookup::new(&mut self.types, self.analysis.model(), self.analysis.api())
    }
}
