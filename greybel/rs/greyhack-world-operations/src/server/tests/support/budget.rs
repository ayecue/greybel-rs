use crate::server::programs::Tree;

/// A program tree.
pub(super) const TREE: Tree = Tree {
    connection: 0,
    root: 0,
};

/// A program tree of another connection.
pub(super) const OTHER: Tree = Tree {
    connection: 1,
    root: 0,
};
