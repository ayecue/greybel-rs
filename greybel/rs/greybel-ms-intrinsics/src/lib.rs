mod error;
pub mod natives;
mod number;

pub use error::IntrinsicError;
use greybel_interpreter::{Api, ValueType, natives};
use greybel_symbols::names::{BIT_AND, BIT_OR, BIT_XOR};
use natives::collections::{
    has_index, index_of_value, indexes, insert, join, len, pop, pull, push, remove, replace,
    shuffle, slice, sort, sum, values,
};
use natives::math::{
    abs, acos, asin, atan, bit_and, bit_or, bit_xor, ceil, cos, floor, log, pi, round, sign, sin,
    sqrt, tan,
};
use natives::range::range;
use natives::strings::{code, from_code_point, lower, split, to_text, upper, val};
pub use natives::system::HostInfo;
use natives::system::{
    end_time_slice, func_ref, hash, intrinsics, list, map, print, ref_equals, rnd, stack_trace,
    string, time, version, wait,
};

pub const API: Api = Api::new()
    .set_globals(natives![
        abs,
        acos,
        asin,
        atan,
        { BIT_AND } => bit_and,
        { BIT_OR } => bit_or,
        { BIT_XOR } => bit_xor,
        "char" => from_code_point,
        ceil,
        code,
        cos,
        floor,
        "funcRef" => func_ref,
        hash,
        "hasIndex" => has_index,
        indexes,
        "indexOf" => index_of_value,
        insert,
        intrinsics,
        join,
        len,
        list,
        log,
        lower,
        map,
        natives::system::number,
        pi,
        print,
        pop,
        pull,
        push,
        range,
        "refEquals" => ref_equals,
        remove,
        replace,
        round,
        rnd,
        sign,
        sin,
        slice,
        sort,
        split,
        sqrt,
        "stackTrace" => stack_trace,
        "str" => to_text,
        string,
        shuffle,
        sum,
        tan,
        time,
        upper,
        val,
        values,
        version,
        wait,
        "yield" => end_time_slice,
    ])
    .set_intrinsics(
        ValueType::List,
        natives![
            "hasIndex" => has_index,
            indexes,
            "indexOf" => index_of_value,
            insert,
            join,
            len,
            pop,
            pull,
            push,
            shuffle,
            sort,
            sum,
            remove,
            replace,
            values,
        ],
    )
    .set_intrinsics(
        ValueType::String,
        natives![
            "hasIndex" => has_index,
            indexes,
            "indexOf" => index_of_value,
            insert,
            code,
            len,
            lower,
            val,
            remove,
            replace,
            split,
            upper,
            values,
        ],
    )
    .set_intrinsics(
        ValueType::Map,
        natives![
            "hasIndex" => has_index,
            indexes,
            "indexOf" => index_of_value,
            len,
            pop,
            push,
            pull,
            shuffle,
            sum,
            remove,
            replace,
            values,
        ],
    );
