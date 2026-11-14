//! Program trees and their connection object tables, reached by ID.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use crate::object::{ObjectTable, ObjectTableHandle};
use crate::program::{Process, ProgramTree};
use crate::protocol::{ObjectId, ProgramId};

/// ID of one live program tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TreeId(pub(crate) u32);

impl TreeId {
    /// Creates an ID allocated by a program-tree pool or server test fixture.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Every live program tree of a world and every connection's shared object table.
pub struct ProgramTreePool<C> {
    trees: RefCell<HashMap<TreeId, ProgramTree<C>>>,
    programs: RefCell<HashMap<ProgramId, TreeId>>,
    objects: RefCell<HashMap<u32, Rc<RefCell<ObjectTable<C>>>>>,
    next: Cell<u32>,
}

impl<C> Default for ProgramTreePool<C> {
    fn default() -> Self {
        Self {
            trees: RefCell::new(HashMap::new()),
            programs: RefCell::new(HashMap::new()),
            objects: RefCell::new(HashMap::new()),
            next: Cell::new(0),
        }
    }
}

impl<C: Copy + PartialEq> ProgramTreePool<C> {
    /// Returns a shared borrow of `id`, or `None` when it has ended.
    pub fn get(&self, id: TreeId) -> Option<Ref<'_, ProgramTree<C>>> {
        Ref::filter_map(self.trees.borrow(), |trees| trees.get(&id)).ok()
    }

    /// Returns a mutable borrow of `id`, or `None` when it has ended.
    pub fn get_mut(&self, id: TreeId) -> Option<RefMut<'_, ProgramTree<C>>> {
        RefMut::filter_map(self.trees.borrow_mut(), |trees| trees.get_mut(&id)).ok()
    }

    /// Returns the tree of `program`, including its termination tombstones.
    #[must_use]
    pub fn of_program(&self, program: ProgramId) -> Option<TreeId> {
        self.programs.borrow().get(&program).copied()
    }

    /// Returns the object table of the tree's connection.
    pub fn objects(&self, id: TreeId) -> ObjectTableHandle<C> {
        let connection = self
            .get(id)
            .map(|tree| tree.connection())
            .expect("an indexed tree exists");

        self.objects_of_connection(connection)
    }

    /// Returns the object table of a connection, including before its first tree starts.
    pub fn objects_of_connection(&self, connection: u32) -> ObjectTableHandle<C> {
        ObjectTableHandle(
            self.objects
                .borrow_mut()
                .entry(connection)
                .or_insert_with(|| Rc::new(RefCell::new(ObjectTable::default())))
                .clone(),
        )
    }

    /// Inserts a root `tree` and indexes its program.
    pub fn insert(&self, tree: ProgramTree<C>) -> TreeId {
        let id = TreeId(self.next.get());

        self.next
            .set(self.next.get().checked_add(1).expect("tree IDs fit u32"));
        self.programs.borrow_mut().insert(tree.root().id, id);
        self.trees.borrow_mut().insert(id, tree);

        id
    }

    /// Pushes a launched program onto `id` and indexes it.
    pub fn push(
        &self,
        id: TreeId,
        program: ProgramId,
        process: Option<Process<C>>,
        session: ObjectId,
        parent: ProgramId,
        kind: crate::ProgramKind<C>,
    ) {
        self.programs.borrow_mut().insert(program, id);
        self.trees
            .borrow_mut()
            .get_mut(&id)
            .expect("the indexed tree exists")
            .push(program, process, session, parent, kind);
    }

    /// Removes a root tree and its program index.
    pub fn remove(&self, id: TreeId) -> Option<ProgramTree<C>> {
        let tree = self.trees.borrow_mut().remove(&id)?;

        self.programs.borrow_mut().retain(|_, owner| *owner != id);

        Some(tree)
    }

    /// Returns trees owned by `connection`.
    #[must_use]
    pub fn on_connection(&self, connection: u32) -> Vec<TreeId> {
        self.trees
            .borrow()
            .iter()
            .filter_map(|(&id, tree)| (tree.connection() == connection).then_some(id))
            .collect()
    }

    /// Returns whether `connection` has any non-tombstone program.
    #[must_use]
    pub fn has_programs(&self, connection: u32) -> bool {
        self.trees
            .borrow()
            .values()
            .any(|tree| tree.connection() == connection && tree.has_programs())
    }

    /// Removes every tree and object of `connection` and returns their active processes.
    pub fn remove_connection(&self, connection: u32) -> Vec<Process<C>> {
        let mut processes = Vec::new();

        for id in self.on_connection(connection) {
            if let Some(tree) = self.remove(id) {
                processes.extend(tree.active_processes());
            }
        }

        self.objects.borrow_mut().remove(&connection);

        processes
    }

    /// Releases objects from `connection`.
    pub fn release(&self, connection: u32, ids: &[ObjectId]) {
        self.objects_of_connection(connection).release(ids);
    }

    /// Interrupts objects whose computer rebooted.
    pub fn interrupt(&self, computer: C) {
        for objects in self.objects.borrow_mut().values_mut() {
            objects.borrow_mut().interrupt(computer);
        }
    }

    /// Returns process locations across all trees for `computers`.
    pub fn processes_on(&self, computers: &[C]) -> Vec<(TreeId, ProgramId, Process<C>)> {
        self.trees
            .borrow()
            .iter()
            .flat_map(|(&id, tree)| {
                tree.processes_on(computers)
                    .into_iter()
                    .map(move |(program, process)| (id, program, process))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Returns the number of registered objects across every connection.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.objects
            .borrow()
            .values()
            .map(|table| table.borrow().len())
            .sum()
    }

    /// Applies `change` to every tree's running mine state.
    pub fn for_each_mut(&self, mut change: impl FnMut(TreeId, &mut ProgramTree<C>)) {
        for (&id, tree) in self.trees.borrow_mut().iter_mut() {
            change(id, tree);
        }
    }
}
