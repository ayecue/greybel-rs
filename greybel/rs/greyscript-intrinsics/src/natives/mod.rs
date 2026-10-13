//! Intrinsics added or modified by Grey Hack, plus language-level [`API`].
//! Unmodified intrinsics come from `greybel_ms_intrinsics`.

pub mod collections;
pub mod math;
pub mod strings;
pub mod system;

use collections::{join, pop, pull, push, remove, replace, reverse, shuffle};
use greybel_interpreter::{Api, ValueType, natives};
use greybel_ms_intrinsics::natives::{
    collections as ms_collections, math as ms_math, strings as ms_strings, system as ms_system,
};
use greybel_symbols::names::{BIT_AND, BIT_OR, BIT_XOR};
use math::{bit_and, bit_or, bit_xor, bitwise, sign};
use strings::{
    code, from_code_point, is_match, last_index_of, matches, md5, replace_regex, split, to_int,
    trim,
};
use system::{hash, print, range, r#typeof, wait};

/// Grey Hack globals and collection methods.
pub const API: Api = Api::new()
    .set_globals(natives![
        ms_math::abs,
        ms_math::acos,
        ms_math::asin,
        ms_math::atan,
        bitwise,
        { BIT_AND } => bit_and,
        { BIT_OR } => bit_or,
        { BIT_XOR } => bit_xor,
        "char" => from_code_point,
        ms_math::ceil,
        code,
        ms_math::cos,
        ms_math::floor,
        "funcRef" => ms_system::func_ref,
        hash,
        "hasIndex" => ms_collections::has_index,
        ms_collections::indexes,
        "indexOf" => ms_collections::index_of_value,
        ms_collections::insert,
        join,
        ms_collections::len,
        ms_system::list,
        ms_math::log,
        ms_strings::lower,
        ms_system::map,
        ms_system::number,
        ms_math::pi,
        print,
        pop,
        pull,
        push,
        range,
        remove,
        replace,
        ms_math::round,
        ms_system::rnd,
        sign,
        ms_math::sin,
        ms_collections::slice,
        ms_collections::sort,
        ms_math::sqrt,
        "str" => ms_strings::to_text,
        ms_system::string,
        shuffle,
        ms_collections::sum,
        ms_math::tan,
        ms_system::time,
        ms_strings::upper,
        ms_strings::val,
        ms_collections::values,
        wait,
        "yield" => ms_system::end_time_slice,
        md5,
        reverse,
        split,
        replace_regex,
        is_match,
        matches,
        trim,
        "lastIndexOf" => last_index_of,
        to_int,
        "typeof" => r#typeof,
    ])
    .set_intrinsics(
        ValueType::List,
        natives![
            "hasIndex" => ms_collections::has_index,
            ms_collections::indexes,
            "indexOf" => ms_collections::index_of_value,
            ms_collections::insert,
            join,
            ms_collections::len,
            pop,
            pull,
            push,
            shuffle,
            ms_collections::sort,
            ms_collections::sum,
            remove,
            replace,
            ms_collections::values,
            reverse,
        ],
    )
    .set_intrinsics(
        ValueType::String,
        natives![
            "hasIndex" => ms_collections::has_index,
            ms_collections::indexes,
            "indexOf" => ms_collections::index_of_value,
            ms_collections::insert,
            code,
            ms_collections::len,
            ms_strings::lower,
            ms_strings::val,
            remove,
            "replace" => replace_regex,
            split,
            ms_strings::upper,
            ms_collections::values,
            to_int,
            trim,
            "lastIndexOf" => last_index_of,
            is_match,
            matches,
        ],
    )
    .set_intrinsics(
        ValueType::Map,
        natives![
            "hasIndex" => ms_collections::has_index,
            ms_collections::indexes,
            "indexOf" => ms_collections::index_of_value,
            ms_collections::len,
            pop,
            push,
            pull,
            shuffle,
            ms_collections::sum,
            remove,
            replace,
            ms_collections::values,
        ],
    );
