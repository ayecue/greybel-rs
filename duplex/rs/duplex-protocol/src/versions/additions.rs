use crate::model::{Field, FieldId, Member, MemberId, Message, Role, Variant, VariantId};

/// Introducing minor version by declaration item, with 0 for initial major-version items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdditionTable {
    fields: Vec<u16>,
    variants: Vec<u16>,
    members: Vec<u16>,
    messages: [Vec<u16>; 2],
}

impl AdditionTable {
    #[must_use]
    pub fn field(&self, field: &Field) -> u16 {
        Self::get(&self.fields, field.id.index())
    }

    #[must_use]
    pub fn variant(&self, variant: &Variant) -> u16 {
        Self::get(&self.variants, variant.id.index())
    }

    #[must_use]
    pub fn member(&self, member: &Member) -> u16 {
        Self::get(&self.members, member.id.index())
    }

    #[must_use]
    pub fn message(&self, message: &Message) -> u16 {
        Self::get(
            &self.messages[usize::from(message.receiver.id())],
            message.id as usize,
        )
    }

    pub(crate) fn set_field(&mut self, id: FieldId, minor: u16) {
        Self::set(&mut self.fields, id.index(), minor);
    }

    pub(crate) fn set_variant(&mut self, id: VariantId, minor: u16) {
        Self::set(&mut self.variants, id.index(), minor);
    }

    pub(crate) fn set_member(&mut self, id: MemberId, minor: u16) {
        Self::set(&mut self.members, id.index(), minor);
    }

    pub(crate) fn set_message(&mut self, receiver: Role, id: u32, minor: u16) {
        Self::set(
            &mut self.messages[usize::from(receiver.id())],
            id as usize,
            minor,
        );
    }

    fn get(minors: &[u16], index: usize) -> u16 {
        minors.get(index).copied().unwrap_or(0)
    }

    fn set(minors: &mut Vec<u16>, index: usize, minor: u16) {
        if minors.len() <= index {
            minors.resize(index + 1, 0);
        }

        minors[index] = minor;
    }
}
