use greybel_transpiler::TranspileOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutoCompile {
    pub allow_import: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOptions {
    pub transpile: TranspileOptions,
    pub ingame_directory: String,
    pub output_name: Option<String>,
    pub auto_compile: Option<AutoCompile>,
}

impl BuildOptions {
    pub const DEFAULT_INGAME_DIRECTORY: &str = "/root/";
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            transpile: TranspileOptions::default(),
            ingame_directory: Self::DEFAULT_INGAME_DIRECTORY.to_owned(),
            output_name: None,
            auto_compile: None,
        }
    }
}
