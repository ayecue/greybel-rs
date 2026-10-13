use std::collections::BTreeSet;

use crate::frame::{Route, SliceKey};

const MAX_ALIAS_KEY: u32 = (1 << 14) - 1;
const MAX_ALIASES: u32 = MAX_ALIAS_KEY / 2;
const _: () = assert!(MAX_ALIASES * 2 <= MAX_ALIAS_KEY);
const FIRST_ALIAS: u32 = 1;

pub(super) struct LocalAliasManager {
    by_route: Vec<(Route, u32)>,
    vacant: BTreeSet<u32>,
    next: u32,
    released_routes: Vec<Route>,
}

impl Default for LocalAliasManager {
    fn default() -> Self {
        Self {
            by_route: Vec::new(),
            vacant: BTreeSet::new(),
            next: FIRST_ALIAS,
            released_routes: Vec::new(),
        }
    }
}

impl LocalAliasManager {
    pub(super) fn key(&mut self, route: Route) -> SliceKey {
        if route.connection.is_none() {
            return SliceKey::NoAlias(route);
        }

        if let Some(&(_, alias)) = self
            .by_route
            .iter()
            .find(|(stored_route, _)| *stored_route == route)
        {
            return SliceKey::Alias(alias);
        }

        let Some(alias) = self.vacant.pop_first().or_else(|| self.take_next()) else {
            return SliceKey::NoAlias(route);
        };

        self.by_route.push((route, alias));

        SliceKey::Announcement(alias, route)
    }

    pub(super) fn release(&mut self, route: Route) {
        self.released_routes.push(route);
    }

    pub(super) fn reclaim_aliases(&mut self) {
        for route in self.released_routes.drain(..) {
            if let Some(index) = self
                .by_route
                .iter()
                .position(|(listed_route, _)| *listed_route == route)
            {
                self.vacant.insert(self.by_route.swap_remove(index).1);
            }
        }
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    fn take_next(&mut self) -> Option<u32> {
        let alias = self.next;

        (alias <= MAX_ALIASES).then(|| {
            self.next += 1;

            alias
        })
    }
}

#[derive(Default)]
pub(super) struct RemoteAliasTable {
    routes: Vec<Option<Route>>,
}

impl RemoteAliasTable {
    pub(super) fn route(&mut self, key: SliceKey) -> Option<Route> {
        match key {
            SliceKey::NoAlias(route) => Some(route),
            SliceKey::Announcement(alias, route) => {
                if alias <= MAX_ALIASES {
                    let index = alias as usize;

                    if index >= self.routes.len() {
                        self.routes.resize(index + 1, None);
                    }

                    self.routes[index] = Some(route);
                }

                Some(route)
            }
            SliceKey::Alias(alias) => self.routes.get(alias as usize).copied().flatten(),
        }
    }

    pub(super) fn clear(&mut self) {
        self.routes.clear();
    }
}
