use greybel_core::PositionEncoding;
use greybel_interpreter::Text;
use greybel_interpreter::value::*;

#[test]
fn should_split_surrogate_pairs_into_utf16_units() {
    let units = EncodedText::new(&Text::new("a😀"), PositionEncoding::Utf16);
    let got = [0, 2, 3].map(|index| units.get(index));

    assert_eq!(got[0].as_deref(), Some("a"));
    assert_eq!(got[1].as_deref(), Some("\u{FFFD}"));
    assert_eq!(got[2], None);
}

#[test]
fn should_read_utf8_units_as_bytes() {
    let units = EncodedText::new(&Text::new("aé😀b"), PositionEncoding::Utf8);
    let got = [7, 8].map(|index| units.get(index));

    assert_eq!(got[0].as_deref(), Some("b"));
    assert_eq!(got[1], None);
}

#[test]
fn should_read_utf32_units_as_characters() {
    let units = EncodedText::new(&Text::new("aé😀b"), PositionEncoding::Utf32);
    let got = [2, 4].map(|index| units.get(index));

    assert_eq!(got[0].as_deref(), Some("😀"));
    assert_eq!(got[1], None);
}

#[test]
fn should_share_the_texts_of_ascii_characters() {
    let text = Text::new("aé😀a");
    let units = EncodedText::new(&text, PositionEncoding::Utf16);
    let ascii = Text::new("abca");
    let (first, last) = (units.get(0), units.get(4));
    let at = ascii.units(PositionEncoding::Utf16).at(3);
    let other = units.get(1);
    let (first, last) = (first.expect("a unit"), last.expect("a unit"));

    assert!(first.ptr_eq(&last));
    assert!(first.ptr_eq(&at));
    assert!(first.is_ascii());

    let other = other.expect("a unit");

    assert_eq!(&*other, "é");
    assert!(!other.is_ascii());
}
