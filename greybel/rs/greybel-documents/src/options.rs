use greybel_core::ParserOptions;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentOptions {
    pub parser: ParserOptions,
    pub unsafe_mode: bool,
    pub native_imports: NativeImportMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NativeImportMode {
    #[default]
    Evaluation,
    Emission,
    Call,
}

impl DocumentOptions {
    #[must_use]
    pub fn parser_options(&self, filename: &str) -> ParserOptions {
        ParserOptions {
            filename: filename.to_owned(),
            ..self.parser.clone()
        }
    }
}
