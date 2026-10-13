//! Converts greybel-js multi-letter short options, including `-fe`, to Clap long-option names.

use std::ffi::OsString;

/// greybel-js multi-letter short options mapped to Clap long options.
const LONG_NAMES: &[(&str, &str)] = &[
    ("-fe", "--file-extensions"),
    ("-ev", "--env-files"),
    ("-vr", "--env-vars"),
    ("-en", "--exclude-namespaces"),
    ("-dlo", "--disable-literals-optimization"),
    ("-dno", "--disable-namespaces-optimization"),
    ("-bkp", "--beautify-keep-parentheses"),
    ("-bi", "--beautify-indentation"),
    ("-bis", "--beautify-indentation-spaces"),
    ("-id", "--ingame-directory"),
    ("-mc", "--max-chars"),
    ("-ac", "--auto-compile"),
    ("-ai", "--allow-import"),
    ("-ci", "--create-ingame"),
    ("-pt", "--port"),
    ("-dbf", "--disable-build-folder"),
    ("-si", "--silent"),
    ("-of", "--output-filename"),
    ("-sm", "--strict-mode"),
    ("-et", "--env-type"),
    ("-pg", "--program-name"),
];

/// Converts greybel-js short options in `arguments` to long names, leaving arguments after `--` unchanged.
pub fn with_long_names(arguments: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut options = true;

    arguments
        .into_iter()
        .map(|argument| {
            if argument == "--" {
                options = false;
            }

            let long_name = options
                .then(|| argument.to_str())
                .flatten()
                .and_then(|text| LONG_NAMES.iter().find(|(short, _)| *short == text));

            long_name.map_or(argument, |(_, long)| OsString::from(long))
        })
        .collect()
}
