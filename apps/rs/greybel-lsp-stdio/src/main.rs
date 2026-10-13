//! Serves the GreyScript language server on stdin/stdout. Rejects arguments except `--stdio`, which is ignored
//! for language-client compatibility.

use std::io;
use std::process::ExitCode;

use greybel_lsp_stdio::Driver;

fn main() -> ExitCode {
    let mut driver = Driver::new(io::stdin().lock(), io::stdout().lock());

    match driver.run() {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            eprintln!("greybel-lsp: {error}");

            ExitCode::FAILURE
        }
    }
}
