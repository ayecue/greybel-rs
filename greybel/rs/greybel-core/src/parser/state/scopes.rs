use crate::ast::{NodeFieldContainer, NodeId, NodeStore, ScopeData};

#[derive(Debug)]
pub struct PendingScope {
    node: NodeId,
    pub namespaces: Vec<NodeId>,
    pub definitions: Vec<NodeId>,
    pub returns: Vec<NodeId>,
}

impl PendingScope {
    fn new(node: NodeId, nodes: &mut NodeStore) -> Self {
        Self {
            node,
            namespaces: nodes.list_buffer(),
            definitions: nodes.list_buffer(),
            returns: nodes.list_buffer(),
        }
    }

    fn resume(node: NodeId, nodes: &mut NodeStore) -> Self {
        let mut scope = Self::new(node, nodes);
        let data = nodes
            .get_mut(node)
            .kind
            .as_scope_mut()
            .map(|scope| std::mem::take(scope.scope_mut()))
            .unwrap_or_default();

        scope
            .namespaces
            .extend_from_slice(nodes.list(data.namespaces));
        scope
            .definitions
            .extend_from_slice(nodes.list(data.definitions));
        scope.returns.extend_from_slice(nodes.list(data.returns));

        scope
    }

    fn finish(self, nodes: &mut NodeStore) {
        let data = ScopeData {
            namespaces: nodes.finish_list(self.namespaces),
            definitions: nodes.finish_list(self.definitions),
            returns: nodes.finish_list(self.returns),
        };

        if let Some(scope) = nodes.get_mut(self.node).kind.as_scope_mut() {
            *scope.scope_mut() = data;
        }
    }

    fn discard(self, nodes: &mut NodeStore) {
        nodes.recycle_list(self.namespaces);
        nodes.recycle_list(self.definitions);
        nodes.recycle_list(self.returns);
    }

    fn lengths(&self) -> [usize; 3] {
        [
            self.namespaces.len(),
            self.definitions.len(),
            self.returns.len(),
        ]
    }

    fn truncate(&mut self, [namespaces, definitions, returns]: [usize; 3]) {
        self.namespaces.truncate(namespaces);
        self.definitions.truncate(definitions);
        self.returns.truncate(returns);
    }
}

#[derive(Debug, Default)]
pub struct ScopeStack {
    scopes: Vec<PendingScope>,
    assignment: Option<NodeId>,
    loops: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScopeStackSnapshot {
    current: Option<NodeId>,
    depth: usize,
    lengths: [usize; 3],
    assignment: Option<NodeId>,
    loops: usize,
}

impl ScopeStack {
    #[must_use]
    pub fn current(&self) -> Option<NodeId> {
        self.scopes.last().map(|scope| scope.node)
    }

    pub fn enter(&mut self, scope: NodeId, nodes: &mut NodeStore) -> bool {
        let nested = !self.scopes.is_empty();

        self.scopes.push(PendingScope::new(scope, nodes));

        nested
    }

    pub fn resume_scope(&mut self, scope: NodeId, nodes: &mut NodeStore) {
        self.scopes.push(PendingScope::resume(scope, nodes));
    }

    pub fn exit(&mut self, nodes: &mut NodeStore) {
        if let Some(scope) = self.scopes.pop() {
            scope.finish(nodes);
        }
    }

    pub fn finish_all(&mut self, nodes: &mut NodeStore) {
        while let Some(scope) = self.scopes.pop() {
            scope.finish(nodes);
        }
    }

    pub fn current_mut(&mut self) -> Option<&mut PendingScope> {
        self.scopes.last_mut()
    }

    pub fn pending_mut(&mut self, scope: NodeId) -> Option<&mut PendingScope> {
        self.scopes
            .iter_mut()
            .rev()
            .find(|pending_scope| pending_scope.node == scope)
    }

    #[must_use]
    pub fn assignment(&self) -> Option<NodeId> {
        self.assignment
    }

    pub fn replace_assignment(&mut self, assignment: Option<NodeId>) -> Option<NodeId> {
        std::mem::replace(&mut self.assignment, assignment)
    }

    #[must_use]
    pub fn innermost_loop(&self) -> Option<NodeId> {
        self.loops.last().copied()
    }

    pub fn enter_loop(&mut self, node: NodeId) {
        self.loops.push(node);
    }

    pub fn exit_loop(&mut self) {
        self.loops.pop();
    }

    pub(crate) fn snapshot(&self) -> ScopeStackSnapshot {
        ScopeStackSnapshot {
            current: self.current(),
            depth: self.scopes.len(),
            lengths: self.scopes.last().map_or([0; 3], PendingScope::lengths),
            assignment: self.assignment,
            loops: self.loops.len(),
        }
    }

    pub(crate) fn restore(&mut self, snapshot: ScopeStackSnapshot, nodes: &mut NodeStore) {
        while self.scopes.len() > snapshot.depth {
            if let Some(scope) = self.scopes.pop() {
                scope.discard(nodes);
            }
        }

        if self.scopes.len() < snapshot.depth
            && let Some(current) = snapshot.current
        {
            self.resume(current, nodes);
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.truncate(snapshot.lengths);
        }

        self.assignment = snapshot.assignment;
        self.loops.truncate(snapshot.loops);
    }

    fn resume(&mut self, current: NodeId, nodes: &mut NodeStore) {
        let outer = self.current();
        let mut chain = vec![current];

        while let Some(parent) = chain.last().and_then(|&scope| nodes.get(scope).scope)
            && Some(parent) != outer
        {
            chain.push(parent);
        }

        for scope in chain.into_iter().rev() {
            self.scopes.push(PendingScope::resume(scope, nodes));
        }
    }
}
