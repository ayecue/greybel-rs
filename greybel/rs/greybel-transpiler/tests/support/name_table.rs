use greybel_transpiler::names::*;

pub fn created(table: &mut NameTable, original: &str) -> String {
    table.create(original).unwrap_or(original).to_owned()
}

/// Creates a `NameTable` with `charset` and `forbidden` names, pre-populated with mappings for `originals`.
pub fn table_with(charset: &[char], forbidden: &[&str], originals: &[&str]) -> NameTable {
    let mut names = NameTable::new(charset.to_vec()).forbid(forbidden.iter().copied());

    for original in originals {
        names.create(original);
    }

    names
}
