use super::*;

pub const LIMIT: usize = 1024;

pub fn after(retained: &Backlog, sequence: u64) -> Vec<(u64, Vec<u8>)> {
    let mut batches = Vec::new();

    retained.each_after(sequence, |sequence, batch| {
        batches.push((sequence, batch.to_vec()));
    });

    batches
}
