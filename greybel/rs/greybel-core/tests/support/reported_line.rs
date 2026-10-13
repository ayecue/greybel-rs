use greybel_core::{Dialect, ParserOptions, parse};

/// The line Grey Hack's parser reports the error in `source` at, which strict GreyScript parsing finds.
pub fn reported_line(source: &str) -> u32 {
    let options = ParserOptions {
        strict_mode: true,
        ..ParserOptions::new(Dialect::GreyScript)
    };

    parse(source, options)
        .unwrap_err()
        .reported_line
        .expect("a reported line")
}
