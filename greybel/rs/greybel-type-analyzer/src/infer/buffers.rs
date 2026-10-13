use crate::types::TypeId;

#[derive(Debug, Default)]
pub(crate) struct TypeBufferPool {
    buffers: Vec<Vec<TypeId>>,
}

impl TypeBufferPool {
    pub(crate) fn take(&mut self) -> Vec<TypeId> {
        self.buffers.pop().unwrap_or_default()
    }

    pub(crate) fn release(&mut self, mut buffer: Vec<TypeId>) {
        buffer.clear();
        self.buffers.push(buffer);
    }
}
