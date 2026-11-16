use greybel_core::NodeId;
use greybel_core::syntax::Return;
use greybel_symbols::Keyword;

use super::Emitter;
use crate::emit::{EmitFlags, EmitState};
use crate::error::Result;

pub(crate) trait EmitStatements: Emitter {
    fn emit_keyword(&mut self, s: &mut EmitState<'_>, keyword: Keyword) -> Result<()> {
        s.keyword(keyword);

        Ok(())
    }

    fn emit_return(&mut self, s: &mut EmitState<'_>, node: &Return) -> Result<()> {
        s.keyword(Keyword::Return);
        s.push(" ");

        match node.argument {
            Some(argument) => self.emit(s, argument, EmitFlags::default()),
            None => Ok(()),
        }
    }

    fn emit_first(&mut self, s: &mut EmitState<'_>, body: &[NodeId]) -> Result<()> {
        match body.first() {
            Some(&item) => self.emit(s, item, EmitFlags::default()),
            None => Ok(()),
        }
    }

    fn emit_statements(&mut self, s: &mut EmitState<'_>, statements: &[NodeId]) -> Result<()> {
        for (index, &statement) in statements.iter().enumerate() {
            if index > 0 {
                self.separate_statements(s);
            }

            self.emit(s, statement, EmitFlags::default())?;
        }

        Ok(())
    }
}

impl<E: Emitter> EmitStatements for E {}
