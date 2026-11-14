use greyhack_world_programs::protocol::{ObjectId, ProgramId};
use greyhack_world_programs::{ProgramKind, ProgramTree, TerminalInfo};

pub(crate) const SESSION: ObjectId = ObjectId { id: 0 };

pub(crate) fn id(id: u32) -> ProgramId {
    ProgramId { id }
}

pub(crate) fn tree() -> ProgramTree<u32> {
    ProgramTree::new(0, id(0), Some((1, 1)), SESSION, false)
}

pub(crate) fn push(tree: &mut ProgramTree<u32>, id: u32, parent: u32, computer: u32) -> ProgramId {
    let program = self::id(id);

    tree.push(
        program,
        Some((computer, id)),
        SESSION,
        self::id(parent),
        ProgramKind::Script,
    );

    program
}

pub(crate) fn terminal(
    tree: &mut ProgramTree<u32>,
    id: u32,
    parent: u32,
    host: u32,
    bounces: u32,
) -> ProgramId {
    let program = self::id(id);

    tree.add_remote_terminal(host);
    tree.push(
        program,
        None,
        SESSION,
        self::id(parent),
        ProgramKind::Terminal(TerminalInfo {
            host,
            bounces,
            launcher_bounces: bounces.saturating_sub(1),
        }),
    );

    program
}
