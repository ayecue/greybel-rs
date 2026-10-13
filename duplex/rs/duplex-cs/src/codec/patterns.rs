use duplex_protocol::PatternTable;

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(super) fn table_bytes(table: &PatternTable) -> Vec<u8> {
    let mut bytes = Vec::new();

    bytes.extend_from_slice(&table.start.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(table.base.len())
            .expect("a pattern of under 4G states")
            .to_le_bytes(),
    );
    bytes.extend_from_slice(
        &u32::try_from(table.next.len())
            .expect("a pattern of under 4G slots")
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&table.classes);
    table
        .base
        .iter()
        .for_each(|base| bytes.extend_from_slice(&base.to_le_bytes()));
    table
        .next
        .iter()
        .for_each(|next| bytes.extend_from_slice(&next.to_le_bytes()));
    table
        .owner
        .iter()
        .for_each(|owner| bytes.extend_from_slice(&owner.to_le_bytes()));
    bytes.extend(table.accepting.iter().map(|&accepts| u8::from(accepts)));

    bytes
}

pub(super) fn base64(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let word = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |word, (index, &byte)| {
                word | u32::from(byte) << (16 - 8 * index)
            });

        for index in 0..4 {
            if index <= chunk.len() {
                text.push(BASE64[(word >> (18 - 6 * index) & 0x3f) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }

    text
}
