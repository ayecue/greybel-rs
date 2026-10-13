//! Parsing of the `>END;result;1` marker printed by SSH encode and decode scripts.

const END_MARKER: &str = ">END;";

/// Server SSH encryption configuration path.
pub const SSHD_CONFIG_PATH: &str = "/server/conf/sshd.conf";
/// Working directory for SSH encode and decode scripts.
pub const SSH_SCRIPT_FOLDER: &str = "/server/conf";
/// First SSH encode/decode script parameter; also reserved as an `rshell_client` process name.
pub const SSH_SCRIPT_PARAM: &str = "ssh_enc";

/// Extracts the result when output contains exactly one complete `>END;result;1` marker, matching
/// `HelperScript.HelperSshSec.Completed`.
#[must_use]
pub fn end_result(output: &str) -> Option<&str> {
    let start = output.find(END_MARKER)?;

    if output.matches(END_MARKER).count() != 1 {
        return None;
    }

    let mut parts = output[start..].split(';');

    if parts.next()? != END_MARKER.trim_end_matches(';') {
        return None;
    }

    let result = parts.next()?;

    (parts.next()? == "1" && parts.next().is_none()).then_some(result)
}

/// Trims output before `>END` for error reporting (`HelperScript.HelperSshSec.GetBgSalida`).
#[must_use]
pub fn before_end(output: &str) -> &str {
    match output.find(END_MARKER.trim_end_matches(';')) {
        Some(index) => output[..index].trim(),
        None => output,
    }
}
