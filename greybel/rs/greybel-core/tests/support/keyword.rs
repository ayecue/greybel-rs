use greybel_core::{Dialect, Keyword, Lexer, Version};

pub fn keywords(source: &str) -> Result<Vec<Keyword>, String> {
    let lexer = Lexer::new(source, Dialect::MiniScript, Version::Latest);
    let mut found = Vec::new();

    for token in lexer.tokens() {
        let token = token.map_err(|error| error.message())?;

        found.extend(token.keyword());
    }

    Ok(found)
}
