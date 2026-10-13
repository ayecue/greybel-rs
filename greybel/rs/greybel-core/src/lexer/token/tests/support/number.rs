use super::NumberShape;

const ALPHABET: [u8; 7] = *b"01.eE+-";

pub(super) fn number_texts(max_len: usize) -> Vec<String> {
    let mut texts = Vec::new();
    let mut current = Vec::new();

    extend(&mut texts, &mut current, max_len);

    texts
}

fn extend(texts: &mut Vec<String>, current: &mut Vec<u8>, max_len: usize) {
    if starts_a_number(current) {
        texts.push(String::from_utf8(current.clone()).expect("the alphabet is ASCII"));
    }

    if current.len() == max_len {
        return;
    }

    for byte in ALPHABET {
        current.push(byte);
        extend(texts, current, max_len);
        current.pop();
    }
}

fn starts_a_number(text: &[u8]) -> bool {
    match text {
        [first, ..] if first.is_ascii_digit() => true,
        [b'.', second, ..] => second.is_ascii_digit(),
        _ => false,
    }
}

pub(super) fn take(text: &str) -> (&str, NumberShape) {
    let mut shape = NumberShape::default();
    let taken = text.bytes().take_while(|&byte| shape.take(byte)).count();

    (&text[..taken], shape)
}
