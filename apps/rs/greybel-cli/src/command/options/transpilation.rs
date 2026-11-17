use clap::{Args, ValueEnum};
use greybel_project::{
    BuildMode, Indentation as Indentations, Manifest, MultilineStringStyle as MultilineStringStyles,
};

/// Build transpilation mode: minified, beautified, unchanged, or obfuscated.
#[derive(Debug, Clone, Args)]
pub struct Transpilation {
    /// Minifies code.
    #[arg(short = 'u', long)]
    pub uglify: bool,
    /// Beautifies code.
    #[arg(short = 'b', long, conflicts_with = "uglify")]
    pub beautify: bool,
    /// Retains all parentheses when beautifying.
    #[arg(long = "beautify-keep-parentheses")]
    pub retain_parentheses: bool,
    /// Set the indentation style for beautified output [default: tab].
    #[arg(long = "beautify-indentation", value_enum, ignore_case = true)]
    pub indentation: Option<Indentation>,
    /// Set the number of spaces per indentation level for whitespace-beautified output [default: 2].
    #[arg(long = "beautify-indentation-spaces")]
    pub indentation_spaces: Option<u32>,
    /// Enable namespace obfuscation using an expanded character set to reduce output size.
    #[arg(short = 'o', long)]
    pub obfuscation: bool,
    /// Disable the literals optimization during uglification.
    #[arg(long = "disable-literals-optimization")]
    pub disable_literals_optimization: bool,
    /// Disable the namespaces optimization during uglification.
    #[arg(long = "disable-namespaces-optimization")]
    pub disable_namespaces_optimization: bool,
    /// Exclude these namespace names from minification.
    #[arg(long = "exclude-namespaces", value_name = "NAMESPACE", num_args = 1..)]
    pub excluded_namespaces: Option<Vec<String>>,
    /// Write strings with line breaks as concatenations with a `char(10)` variable, or as a joining function call
    /// [default: concatenation].
    #[arg(long = "multiline-strings", value_enum, ignore_case = true)]
    pub multiline_strings: Option<MultilineStringStyle>,
}

/// How builds write strings with line breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MultilineStringStyle {
    Concatenation,
    Join,
}

/// Indentation styles available for beautified output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Indentation {
    Tab,
    Whitespace,
}

impl Transpilation {
    pub fn apply(&self, manifest: &mut Manifest) {
        let build = &mut manifest.build;
        let format = &mut manifest.format;

        if self.uglify {
            build.mode = BuildMode::Uglify;
        } else if self.beautify {
            build.mode = BuildMode::Beautify;
        }

        format.retain_parentheses |= self.retain_parentheses;

        if let Some(indentation) = self.indentation {
            format.indentation = match indentation {
                Indentation::Tab => Indentations::Tab,
                Indentation::Whitespace => Indentations::Whitespace,
            };
        }

        if let Some(spaces) = self.indentation_spaces {
            format.indentation_spaces = spaces;
        }

        build.obfuscation |= self.obfuscation;
        build.disable_literals_optimization |= self.disable_literals_optimization;
        build.disable_namespaces_optimization |= self.disable_namespaces_optimization;

        if let Some(names) = &self.excluded_namespaces {
            build.excluded_namespaces.clone_from(names);
        }

        if let Some(style) = self.multiline_strings {
            build.multiline_strings = match style {
                MultilineStringStyle::Concatenation => MultilineStringStyles::Concatenation,
                MultilineStringStyle::Join => MultilineStringStyles::Join,
            };
        }
    }
}
