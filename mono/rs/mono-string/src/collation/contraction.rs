#[derive(Debug)]
pub(super) struct Contraction {
    pub(super) index: usize,
    pub(super) source: Vec<u16>,
    pub(super) replacement: Option<Vec<u16>>,
    pub(super) sort_key: Option<[u8; 4]>,
}

pub(super) fn contractions(tailoring: &[u16], len: usize) -> Vec<Contraction> {
    let text = |from: usize| -> (Vec<u16>, usize) {
        let end = from
            + tailoring[from..]
                .iter()
                .position(|&unit| unit == 0)
                .expect("NUL");

        (tailoring[from..end].to_vec(), end)
    };
    let mut contractions = Vec::new();
    let mut at = 0;

    while at < len {
        match tailoring[at] {
            1 => {
                let (source, end) = text(at + 1);
                let mut sort_key = [0; 4];

                for (slot, &unit) in sort_key.iter_mut().zip(&tailoring[end + 1..end + 5]) {
                    *slot = unit as u8;
                }

                contractions.push(Contraction {
                    index: contractions.len(),
                    source,
                    replacement: None,
                    sort_key: Some(sort_key),
                });
                at = end + 6;
            }
            2 => at += 3,
            3 => {
                let (source, end) = text(at + 1);
                let (replacement, end) = text(end + 1);

                contractions.push(Contraction {
                    index: contractions.len(),
                    source,
                    replacement: Some(replacement),
                    sort_key: None,
                });
                at = end + 1;
            }
            other => panic!("collation tailoring table is broken: entry {other} at {at}"),
        }
    }

    contractions.sort_by(|a, b| a.source.cmp(&b.source).then(a.index.cmp(&b.index)));

    contractions
}
