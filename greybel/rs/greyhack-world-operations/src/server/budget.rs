//! Per-tree token buckets share world access between programs. Buckets refill each tick. Empty buckets
//! wait only while another claimant needs the world; an uncontended program continues without waiting.
//! A tree that waits is throttled after its operation response so its runtime pauses execution.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use greyhack_world_router::TreeId;
use tokio::sync::Notify;

type Tree = TreeId;

/// World claimant: a program tree or world routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Owner {
    Tree(Tree),
    World,
}

/// Tree token balances and active world claims by owner.
#[derive(Default)]
pub(super) struct BudgetTable {
    tokens: RefCell<HashMap<Tree, u32>>,
    claims: RefCell<HashMap<Owner, usize>>,
    refill: Notify,
    /// Trees awaiting a throttle notification after waiting for a refill.
    throttles: RefCell<HashSet<Tree>>,
}

impl BudgetTable {
    /// Registers `claimant` until the returned claim is dropped.
    pub(super) fn claim(&self, claimant: Owner) -> Claim<'_> {
        *self.claims.borrow_mut().entry(claimant).or_default() += 1;

        Claim {
            table: self,
            claimant,
        }
    }

    /// Checks for active claims from owners other than `claimant`.
    fn has_contention(&self, claimant: Owner) -> bool {
        self.claims
            .borrow()
            .iter()
            .any(|(&other, &count)| other != claimant && count > 0)
    }

    /// Consumes one token from `tree`, whose capacity is `cap`. Under contention, an empty bucket waits
    /// for a refill and marks the tree for throttling (see [`BudgetTable::take_throttle`]).
    pub(super) async fn draw(&self, tree: Tree, cap: u32) {
        loop {
            let refill = self.refill.notified();

            {
                let mut tokens = self.tokens.borrow_mut();
                let bucket = tokens.entry(tree).or_insert(cap);

                if *bucket > 0 {
                    *bucket -= 1;

                    return;
                }
            }

            if !self.has_contention(Owner::Tree(tree)) {
                return;
            }

            self.throttles.borrow_mut().insert(tree);
            refill.await;
        }
    }

    /// Refills every bucket by `tokens` for each of `ticks` ticks, capped at `cap`.
    pub(super) fn refill(&self, ticks: u64, tokens: u32, cap: u32) {
        let amount = u32::try_from(ticks.saturating_mul(u64::from(tokens))).unwrap_or(u32::MAX);

        for bucket in self.tokens.borrow_mut().values_mut() {
            *bucket = bucket.saturating_add(amount).min(cap);
        }

        self.refill.notify_waiters();
    }

    /// Returns and clears whether `tree` waited for a refill since the last call.
    pub(super) fn take_throttle(&self, tree: Tree) -> bool {
        self.throttles.borrow_mut().remove(&tree)
    }

    /// Removes budget and throttle state for `trees`.
    pub(super) fn remove(&self, trees: &[Tree]) {
        self.tokens
            .borrow_mut()
            .retain(|tree, _| !trees.contains(tree));
        self.throttles
            .borrow_mut()
            .retain(|tree| !trees.contains(tree));
    }
}

/// Active world claim released on drop.
pub(super) struct Claim<'t> {
    table: &'t BudgetTable,
    claimant: Owner,
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        let mut claims = self.table.claims.borrow_mut();

        if let Some(count) = claims.get_mut(&self.claimant) {
            *count -= 1;

            if *count == 0 {
                claims.remove(&self.claimant);
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/budget.rs"]
mod tests;
