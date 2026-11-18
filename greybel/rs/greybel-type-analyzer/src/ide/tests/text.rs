use super::SourceText;

#[test]
fn should_read_the_chain_before_a_dot() {
    let source = "x = shell.host_computer.";

    let chain = SourceText::new(source).member_chain_before(source.len() - 1);
    let after_a_call = SourceText::new("f().b").member_chain_before(5);

    assert_eq!(
        chain,
        Some(vec!["shell".to_owned(), "host_computer".to_owned()])
    );
    assert_eq!(after_a_call, None);
}

#[test]
fn should_find_where_a_name_starts() {
    let source = SourceText::new("abc de");

    let start = source.name_start(6);

    assert_eq!(start, 4);
}

#[test]
fn should_find_where_a_word_starts_after_a_multibyte_character() {
    let source = SourceText::new("x = ¶ab");

    let after_name = source.name_start(8);
    let after_symbol = SourceText::new("x = ¶").name_start(6);
    let tag = SourceText::new("// ¶@para-m").word_start(12, |c| c.is_alphanumeric() || c == '-');

    assert_eq!(after_name, 6);
    assert_eq!(after_symbol, 6);
    assert_eq!(tag, 6);
}

#[test]
fn should_find_the_open_call_and_its_commas() {
    let source = "print foo(1, [2, 3], \"a,b\", ";

    let call = SourceText::new(source).open_call(source.len());
    let nested = SourceText::new("print foo(bar(1").open_call(15);
    let in_a_list = SourceText::new("x = [1, ").open_call(8);

    assert_eq!(call, Some((9, 3)));
    assert_eq!(nested, Some((13, 0)));
    assert_eq!(in_a_list, None);
}
