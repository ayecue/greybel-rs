#![allow(unsafe_code)]

use std::ptr;

use super::mono_regex_free;

/// Maps to `Result` in `mono/dotnet/mono-regex/src/Result.cs`.
#[repr(C)]
pub(super) struct RawOutput {
    status: i32,
    is_match: i32,
    text: *mut u16,
    text_length: i32,
    numbers: *mut i32,
    numbers_length: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Timeout,
    Failure,
}

/// Call output copied from the library's buffers.
pub struct Output {
    pub status: Status,
    pub is_match: bool,
    pub text: Vec<u16>,
    pub numbers: Vec<i32>,
}

pub(super) fn call(call: impl FnOnce(*mut RawOutput)) -> Output {
    let mut raw = RawOutput {
        status: 0,
        is_match: 0,
        text: ptr::null_mut(),
        text_length: 0,
        numbers: ptr::null_mut(),
        numbers_length: 0,
    };

    call(&raw mut raw);

    let output = unsafe {
        Output {
            status: match raw.status {
                0 => Status::Ok,
                1 => Status::Timeout,
                _ => Status::Failure,
            },
            is_match: raw.is_match != 0,
            text: copy(raw.text, raw.text_length),
            numbers: copy(raw.numbers, raw.numbers_length),
        }
    };

    unsafe { mono_regex_free(&raw mut raw) };

    output
}

/// # Safety
///
/// `data` must point to `length` initialized values, or `length` must be 0.
unsafe fn copy<T: Copy>(data: *const T, length: i32) -> Vec<T> {
    match usize::try_from(length) {
        Ok(length) if length > 0 && !data.is_null() => {
            unsafe { std::slice::from_raw_parts(data, length) }.to_vec()
        }
        _ => Vec::new(),
    }
}
