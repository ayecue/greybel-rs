use crate::ast::NodeId;

pub(super) fn id(index: usize) -> NodeId {
    NodeId::from_index(index)
}

pub(super) fn ranges() -> Vec<(u32, u32, NodeId)> {
    let mut rng = fastrand::Rng::with_seed(1);
    let mut items = vec![
        (0, 3, id(0)),
        (1, 1, id(1)),
        (2, 9, id(2)),
        (5, 5, id(3)),
        (4, 17, id(4)),
    ];

    for index in items.len()..200 {
        let start = rng.u32(..120);
        let len = rng.u32(..40);

        items.push((start, start + len, id(index)));
    }

    items
}

pub(super) fn containing(items: &[(u32, u32, NodeId)], line: u32) -> Vec<NodeId> {
    let mut expected: Vec<_> = items
        .iter()
        .filter(|&&(start, end, _)| line >= 1 && start <= line && line <= end)
        .map(|&(_, _, id)| id)
        .collect();

    expected.sort();

    expected
}
