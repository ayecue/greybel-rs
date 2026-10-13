//! C ABI for the Native AOT library (`mono/dotnet/mono-regex/src/Exports.cs`).
#![allow(unsafe_code)]

mod output;
mod utf16;

pub use output::{Output, Status};
use output::{RawOutput, call};
pub use utf16::Utf16;

#[cfg_attr(windows, link(name = "MonoRegex", kind = "raw-dylib"))]
unsafe extern "C" {
    fn mono_regex_is_match(
        input: *const u16,
        input_length: i32,
        pattern: *const u16,
        pattern_length: i32,
        options: i32,
        timeout_milliseconds: i32,
        result: *mut RawOutput,
    );

    fn mono_regex_matches(
        input: *const u16,
        input_length: i32,
        pattern: *const u16,
        pattern_length: i32,
        options: i32,
        timeout_milliseconds: i32,
        result: *mut RawOutput,
    );

    fn mono_regex_split(
        input: *const u16,
        input_length: i32,
        pattern: *const u16,
        pattern_length: i32,
        options: i32,
        timeout_milliseconds: i32,
        result: *mut RawOutput,
    );

    #[allow(clippy::too_many_arguments)]
    fn mono_regex_replace(
        input: *const u16,
        input_length: i32,
        pattern: *const u16,
        pattern_length: i32,
        replacement: *const u16,
        replacement_length: i32,
        options: i32,
        timeout_milliseconds: i32,
        result: *mut RawOutput,
    );

    fn mono_regex_free(result: *mut RawOutput);
}

pub fn is_match(input: &Utf16, pattern: &Utf16, options: i32, timeout: i32) -> Output {
    call(|output| unsafe {
        mono_regex_is_match(
            input.units.as_ptr(),
            input.length,
            pattern.units.as_ptr(),
            pattern.length,
            options,
            timeout,
            output,
        );
    })
}

pub fn matches(input: &Utf16, pattern: &Utf16, options: i32, timeout: i32) -> Output {
    call(|output| unsafe {
        mono_regex_matches(
            input.units.as_ptr(),
            input.length,
            pattern.units.as_ptr(),
            pattern.length,
            options,
            timeout,
            output,
        );
    })
}

pub fn split(input: &Utf16, pattern: &Utf16, options: i32, timeout: i32) -> Output {
    call(|output| unsafe {
        mono_regex_split(
            input.units.as_ptr(),
            input.length,
            pattern.units.as_ptr(),
            pattern.length,
            options,
            timeout,
            output,
        );
    })
}

pub fn replace(
    input: &Utf16,
    pattern: &Utf16,
    replacement: &Utf16,
    options: i32,
    timeout: i32,
) -> Output {
    call(|output| unsafe {
        mono_regex_replace(
            input.units.as_ptr(),
            input.length,
            pattern.units.as_ptr(),
            pattern.length,
            replacement.units.as_ptr(),
            replacement.length,
            options,
            timeout,
            output,
        );
    })
}
