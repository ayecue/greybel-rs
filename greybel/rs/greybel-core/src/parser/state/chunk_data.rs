use crate::ast::{Comment, NodeId};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChunkData {
    pub literals: Vec<NodeId>,
    pub comments: Vec<Comment>,
    pub scopes: Vec<NodeId>,
    pub line_items: Vec<NodeId>,
    pub imports: Vec<NodeId>,
    pub includes: Vec<NodeId>,
    pub injects: Vec<NodeId>,
    pub native_imports: Vec<NodeId>,
    pub debuggers: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkDataSnapshot {
    literals: usize,
    comments: usize,
    scopes: usize,
    line_items: usize,
    imports: usize,
    includes: usize,
    injects: usize,
    native_imports: usize,
    debuggers: usize,
}

impl ChunkData {
    #[must_use]
    pub fn snapshot(&self) -> ChunkDataSnapshot {
        ChunkDataSnapshot {
            literals: self.literals.len(),
            comments: self.comments.len(),
            scopes: self.scopes.len(),
            line_items: self.line_items.len(),
            imports: self.imports.len(),
            includes: self.includes.len(),
            injects: self.injects.len(),
            native_imports: self.native_imports.len(),
            debuggers: self.debuggers.len(),
        }
    }

    pub fn restore(&mut self, snapshot: ChunkDataSnapshot) {
        self.literals.truncate(snapshot.literals);
        self.comments.truncate(snapshot.comments);
        self.scopes.truncate(snapshot.scopes);
        self.line_items.truncate(snapshot.line_items);
        self.imports.truncate(snapshot.imports);
        self.includes.truncate(snapshot.includes);
        self.injects.truncate(snapshot.injects);
        self.native_imports.truncate(snapshot.native_imports);
        self.debuggers.truncate(snapshot.debuggers);
    }

    pub fn merge(&mut self, other: Self) {
        self.literals.extend(other.literals);
        self.comments.extend(other.comments);
        self.scopes.extend(other.scopes);
        self.line_items.extend(other.line_items);
        self.imports.extend(other.imports);
        self.includes.extend(other.includes);
        self.injects.extend(other.injects);
        self.native_imports.extend(other.native_imports);
        self.debuggers.extend(other.debuggers);
    }
}

#[derive(Default)]
pub struct ChunkDataStack {
    base: ChunkData,
    inner: Vec<ChunkData>,
}

impl ChunkDataStack {
    pub fn current_mut(&mut self) -> &mut ChunkData {
        self.inner.last_mut().unwrap_or(&mut self.base)
    }

    fn current(&self) -> &ChunkData {
        self.inner.last().unwrap_or(&self.base)
    }

    pub fn push(&mut self, chunk_data: ChunkData) {
        self.inner.push(chunk_data);
    }

    pub fn pop(&mut self) -> Option<ChunkData> {
        self.inner.pop()
    }

    pub(crate) fn snapshot(&self) -> ChunkDataSnapshot {
        self.current().snapshot()
    }

    pub(crate) fn restore(&mut self, snapshot: ChunkDataSnapshot) {
        self.current_mut().restore(snapshot);
    }

    pub(crate) fn into_base(self) -> ChunkData {
        self.base
    }
}
