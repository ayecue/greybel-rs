use greybel_core::parser::*;

#[test]
fn should_keep_the_order_when_editing_the_table() {
    let extra = Precedence::Shift;
    let table = PrecedenceTable::new(&[Precedence::Or, Precedence::And, Precedence::Call]);

    let table = table
        .insert_after(Precedence::Or, &[extra])
        .swap(Precedence::And, Precedence::Call);

    assert_eq!(
        table.levels(),
        [Precedence::Or, extra, Precedence::Call, Precedence::And]
    );
    assert_eq!(table.rank(Precedence::Or), Some(1));
    assert_eq!(table.rank(Precedence::And), Some(4));
    assert_eq!(table.rank(Precedence::Power), None);
}
