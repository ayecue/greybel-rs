//! Builds a launched program into binary source, matching `greybel build` with `import_code` dependencies in place.

use greybel_core::{Dialect, SyntaxFeatures};
use greybel_documents::{AsyncResourceProvider, Uri};
use greybel_source_map::SourceMap;
use greybel_transpiler::{MultilineStringStyle as TranspiledStyle, TranspileOptions, Transpiler};
use greyhack_behaviour::{Import, put_imports};
use greyscript_intrinsics::ProgramBinary;

use crate::protocol::{Launch, LaunchFailure, MultilineStringStyle};
use crate::uri::EditorUri as _;

/// Builds `launch` into a binary from `provider` files, with source-map filenames set to URIs. Retains `debugger`
/// statements when `retain_debugger` is true.
pub(crate) async fn build(
    launch: &Launch<'static>,
    provider: &impl AsyncResourceProvider,
    retain_debugger: bool,
) -> Result<ProgramBinary, LaunchFailure> {
    let entry = provider
        .resolve(&launch.entry.canonical())
        .await
        .map_err(|error| LaunchFailure::invalid(error.to_string()))?;
    let options = TranspileOptions {
        dialect: Dialect::GreyScript,
        environment: launch
            .variables
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        strict_mode: launch.strict_mode,
        syntax_features: SyntaxFeatures::except(
            launch
                .disabled_syntax
                .iter()
                .flat_map(|names| names.iter())
                .map(String::as_str),
        ),
        obfuscation: false,
        retain_debugger,
        multiline_strings: match launch.multiline_strings {
            Some(MultilineStringStyle::Join) => TranspiledStyle::Join,
            Some(MultilineStringStyle::Concatenation) => TranspiledStyle::Concatenation,
            Some(MultilineStringStyle::Replace) | None => TranspiledStyle::Replace,
        },
        ..TranspileOptions::default()
    };
    let mut outputs = Transpiler::new(&options)
        .transpile_project_async(&entry, provider)
        .await
        .map_err(|error| LaunchFailure::invalid(error.to_string()))?
        .into_iter();
    let Some(main) = outputs.next() else {
        return Ok(ProgramBinary {
            code: String::new(),
            source_map: SourceMap::default(),
        });
    };
    let imports: Vec<Import> = outputs
        .map(|output| Import {
            path: output.target.label(),
            code: output.code,
            source_map: output.source_map,
        })
        .collect();
    let (code, source_map) =
        put_imports(main.code, main.source_map, &imports, None).expect("code without a limit fits");

    Ok(ProgramBinary { code, source_map })
}

/// Returns `binary` with each source-map filename transformed by `name` from its URI.
pub(crate) fn name_sources(binary: ProgramBinary, name: impl Fn(&Uri) -> String) -> ProgramBinary {
    ProgramBinary {
        source_map: binary.source_map.map_sources(|source| {
            Uri::parse(source).map_or_else(|_| source.to_owned(), |uri| name(&uri))
        }),
        ..binary
    }
}
