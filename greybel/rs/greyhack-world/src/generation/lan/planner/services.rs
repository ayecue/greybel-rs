//! Drawing the services of the network pools (`NetworkLan.GetRandomServices`).

use greyhack_behaviour::{NetworkKind, ServiceId};

use super::Planner;
use crate::generation::lan::{Pool, PoolService};

/// Returns whether `service` can run on multiple servers (`CanRepeatService`).
pub(super) fn repeatable(service: ServiceId, kind: NetworkKind) -> bool {
    match service {
        ServiceId::Ftp => true,
        ServiceId::Ssh => kind != NetworkKind::RentServer,
        _ => false,
    }
}

/// Returns whether `service` is a main service restricted to router computers (`CanRepeatService`).
pub(super) fn main_service(service: ServiceId, kind: NetworkKind) -> bool {
    match service {
        ServiceId::Http | ServiceId::Repository => true,
        ServiceId::Ssh => kind == NetworkKind::RentServer,
        _ => false,
    }
}

impl Planner<'_> {
    /// `GeneraServiciosServer`: selects guaranteed and random public services shared by network
    /// servers using a unique-entry `RDSTable`.
    pub(super) fn draw_services(&mut self, table: &[(ServiceId, u8, bool)]) -> Vec<PoolService> {
        let entries: Vec<PoolService> = table
            .iter()
            .map(|entry| PoolService {
                service: entry.0,
                chance: entry.1,
                closed: entry.2,
            })
            .collect();
        let sure = entries.iter().filter(|entry| entry.chance == 100).count() as u32;
        let count = self.next(sure, entries.len() as u32 + 1);
        let mut result: Vec<PoolService> = entries
            .iter()
            .copied()
            .filter(|entry| entry.chance == 100)
            .collect();
        let by_chance: Vec<PoolService> = entries
            .iter()
            .copied()
            .filter(|entry| entry.chance != 100)
            .collect();
        let total: u32 = by_chance.iter().map(|entry| u32::from(entry.chance)).sum();

        for _ in sure..count {
            if total == 0 {
                break;
            }

            let hit = self.next(0, total);
            let mut sum = 0;

            for entry in &by_chance {
                sum += u32::from(entry.chance);

                if hit < sum {
                    // Duplicate results are discarded (`RDSTable.AddToResult`).
                    if !result.contains(entry) {
                        result.push(*entry);
                    }

                    break;
                }
            }
        }

        result
    }

    pub(super) fn pool(&mut self, pool: Pool) -> &mut Vec<PoolService> {
        match pool {
            Pool::Public => &mut self.public,
            Pool::Private => &mut self.private,
        }
    }
}
