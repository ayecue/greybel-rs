use rustc_hash::FxHashSet;

use crate::model::{Site, SlotId, SlotList};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferenceTable {
    node_entries: Vec<Vec<u32>>,
    site_slots: Vec<(Site, SlotList)>,
    definition_sites: Vec<Vec<Site>>,
    defined_slots: Vec<SlotId>,
}

impl ReferenceTable {
    pub(crate) fn new(node_counts: impl IntoIterator<Item = usize>) -> Self {
        Self {
            node_entries: node_counts
                .into_iter()
                .map(|node_entries| vec![0; node_entries])
                .collect(),
            site_slots: Vec::new(),
            definition_sites: Vec::new(),
            defined_slots: Vec::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        for (site, _) in self.site_slots.drain(..) {
            self.node_entries[site.document as usize][site.node.index()] = 0;
        }

        for slot in self.defined_slots.drain(..) {
            self.definition_sites[slot.index()].clear();
        }
    }

    pub(crate) fn add_reference(&mut self, site: Site, slots: &[SlotId]) {
        if slots.is_empty() {
            return;
        }

        let node_entries = &mut self.node_entries[site.document as usize];
        let node = site.node.index();

        if node_entries.len() <= node {
            node_entries.resize(node + 1, 0);
        }

        let entry = match node_entries[node] {
            0 => {
                self.site_slots.push((site, SlotList::Empty));
                node_entries[node] =
                    u32::try_from(self.site_slots.len()).expect("fewer than 2^32 references");

                self.site_slots.len() - 1
            }
            entry => entry as usize - 1,
        };

        self.site_slots[entry].1.extend(slots);
    }

    pub(crate) fn add_definition(&mut self, site: Site, slots: &[SlotId]) {
        self.add_reference(site, slots);
        for &slot in slots {
            if self.definition_sites.len() <= slot.index() {
                self.definition_sites
                    .resize_with(slot.index() + 1, Vec::new);
            }

            let definition_sites = &mut self.definition_sites[slot.index()];

            if definition_sites.is_empty() {
                self.defined_slots.push(slot);
            }

            if !definition_sites.contains(&site) {
                definition_sites.push(site);
            }
        }
    }

    #[must_use]
    pub fn slots(&self, site: Site) -> &[SlotId] {
        self.node_entries
            .get(site.document as usize)
            .and_then(|node_entries| node_entries.get(site.node.index()))
            .filter(|&&entry| entry != 0)
            .map_or(&[], |&entry| &self.site_slots[entry as usize - 1].1)
    }

    #[must_use]
    pub fn definition_sites(&self, slot: SlotId) -> &[Site] {
        self.definition_sites
            .get(slot.index())
            .map_or(&[], Vec::as_slice)
    }

    pub fn find_referring_sites<'s>(
        &'s self,
        slots: &'s [SlotId],
    ) -> impl Iterator<Item = Site> + 's {
        self.entries()
            .filter(|(_, site_slots)| site_slots.iter().any(|slot| slots.contains(slot)))
            .map(|(site, _)| site)
    }

    pub(crate) fn read_slots(&self) -> FxHashSet<SlotId> {
        let mut read = FxHashSet::default();

        for (site, slots) in self.entries() {
            for &slot in slots {
                if !self.definition_sites(slot).contains(&site) {
                    read.insert(slot);
                }
            }
        }

        read
    }

    fn entries(&self) -> impl Iterator<Item = (Site, &[SlotId])> {
        self.site_slots
            .iter()
            .map(|(site, slots)| (*site, &**slots))
    }
}
