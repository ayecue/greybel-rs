use super::*;

#[test]
fn should_flatten_and_sort_union_variants() {
    let mut table = TypeTable::default();
    let list = table.list(TypeId::NUMBER);
    let left = table.union(TypeId::STRING, list);
    let right = table.union(list, TypeId::NUMBER);

    let all = table.union(left, right);
    let variants: Vec<_> = table.variants(all).collect();

    assert_eq!(variants, [TypeId::NUMBER, TypeId::STRING, list]);
}

#[test]
fn should_unite_lists_into_a_list_of_the_united_items() {
    let mut table = TypeTable::default();
    let list = table.list(TypeId::NUMBER);
    let strings = table.list(TypeId::STRING);

    let lists = table.union(list, strings);
    let items = table.union(TypeId::NUMBER, TypeId::STRING);
    let list_of_items = table.list(items);

    assert_eq!(lists, list_of_items);
}

#[test]
fn should_absorb_variants_into_a_union_or_any() {
    let mut table = TypeTable::default();
    let list = table.list(TypeId::NUMBER);
    let left = table.union(TypeId::STRING, list);
    let right = table.union(list, TypeId::NUMBER);
    let all = table.union(left, right);

    let with_a_variant = table.union(all, TypeId::NUMBER);
    let with_any = table.union(all, TypeId::ANY);
    let empty = table.union_all([]);
    let unknown_and_null = table.union(TypeId::UNKNOWN, TypeId::NULL);

    assert_eq!(with_a_variant, all);
    assert_eq!(with_any, TypeId::ANY);
    assert_eq!(empty, TypeId::UNKNOWN);
    assert_eq!(unknown_and_null, TypeId::NULL);
}

#[test]
fn should_list_a_single_type_as_its_only_variant() {
    let table = TypeTable::default();

    let variants = table.variants(TypeId::NULL).collect::<Vec<_>>();

    assert_eq!(variants, [TypeId::NULL]);
}

#[test]
fn should_collapse_deep_nesting() {
    let mut table = TypeTable::default();
    let mut ty = TypeId::NUMBER;

    for _ in 0..20 {
        ty = table.list(ty);
    }

    let mut depth = 0;

    while let Type::List(inner) = table.get(ty) {
        depth += 1;
        ty = *inner;
    }

    assert_eq!(depth, usize::from(MAX_DEPTH));
    assert_eq!(ty, TypeId::ANY);
}
