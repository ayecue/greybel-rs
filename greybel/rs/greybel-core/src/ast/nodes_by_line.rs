use super::NodeId;

#[derive(Debug, Clone, Default)]
pub struct NodesByLine {
    leaf_count: usize,
    buckets: Vec<Vec<NodeId>>,
}

fn segments(leaf_count: usize, start: u32, end: u32) -> impl Iterator<Item = usize> {
    let mut lo = start as usize + leaf_count;
    let mut hi = end as usize + leaf_count + 1;
    let mut pending_slot = None;

    std::iter::from_fn(move || {
        loop {
            if let Some(slot) = pending_slot.take() {
                return Some(slot);
            }

            if lo >= hi {
                return None;
            }

            let left = (lo & 1 == 1).then(|| {
                lo += 1;

                lo - 1
            });

            if hi & 1 == 1 {
                hi -= 1;
                pending_slot = Some(hi);
            }

            lo >>= 1;
            hi >>= 1;
            if left.is_some() {
                return left;
            }
        }
    })
}

impl NodesByLine {
    pub fn new(items: impl IntoIterator<Item = (u32, u32, NodeId)>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        let Some(max_line) = items.iter().map(|&(_, end, _)| end).max() else {
            return Self::default();
        };
        let leaf_count = (max_line as usize + 1).next_power_of_two();
        let mut buckets = vec![Vec::new(); leaf_count * 2];

        for (start, end, id) in items {
            for slot in segments(leaf_count, start, end) {
                buckets[slot].push(id);
            }
        }

        Self {
            leaf_count,
            buckets,
        }
    }

    #[must_use]
    pub fn get(&self, line: u32) -> Vec<NodeId> {
        let line = line as usize;

        if line < 1 || line >= self.leaf_count {
            return Vec::new();
        }

        std::iter::successors(Some(line + self.leaf_count), |&slot| {
            (slot > 1).then_some(slot / 2)
        })
        .flat_map(|slot| self.buckets[slot].iter().copied())
        .collect()
    }
}

#[cfg(test)]
#[path = "tests/nodes_by_line.rs"]
mod tests;
