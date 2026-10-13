use super::super::translate;
use crate::RegexOptions;
use crate::native::error::Reason;

pub(super) const NONE: RegexOptions = RegexOptions::empty();

pub(super) fn source(pattern: &str, options: RegexOptions) -> String {
    translate(pattern, options).expect("translates").source
}

pub(super) fn reason(pattern: &str) -> Reason {
    translate(pattern, RegexOptions::empty())
        .expect_err("rejected")
        .reason
}
