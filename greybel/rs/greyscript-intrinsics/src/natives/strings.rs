use greybel_interpreter::{EncodedText, PrintOptions, Value, Vm, native};
use greyhack_behaviour::hash_password;
use mono_number::NumberStyles;

use crate::error::IntrinsicError;
use crate::game::LoginEncryption;
use crate::regex::{HostRegexEngine, RegexError, RegexOptions, parse_options};

/// Grey Hack restricts character codes to the Basic Multilingual Plane.
const MAX_CHAR_CODE: u32 = 0xFFFF;
const DOT: &str = ".";
const DOT_WARNING: &str =
    "<color=yellow>Warning: to split using the dot character you must use split(\"\\.\")</color>";

fn is_valid_char(code: u32) -> bool {
    code <= MAX_CHAR_CODE && char::from_u32(code).is_some()
}

#[native]
pub fn from_code_point(
    #[default(65)]
    #[name = "codePoint"]
    code_point: i32,
) -> Result<String, IntrinsicError> {
    u32::try_from(code_point)
        .ok()
        .filter(|&code| is_valid_char(code))
        .and_then(char::from_u32)
        .map(String::from)
        .ok_or(IntrinsicError::CharInvalid)
}

/// Returns the first character's code point.
/// Characters outside the Basic Multilingual Plane are errors.
#[native(method)]
pub fn code(vm: &mut Vm, this: &Value) -> Result<f64, IntrinsicError> {
    if this.is_null() {
        return Ok(0.0);
    }

    let text = vm.heap().display(this).to_string();
    let first = text.chars().next().ok_or(IntrinsicError::CodeEmpty)?;
    let code = u32::from(first);

    if !is_valid_char(code) {
        return Err(IntrinsicError::CodeInvalid);
    }

    Ok(f64::from(code))
}

/// Returns lowercase hexadecimal MD5 of the text's UTF-8 bytes.
#[native]
pub fn md5(vm: &mut Vm, value: &Value) -> Result<String, IntrinsicError> {
    LoginEncryption::deny_in(vm, "md5")?;

    value
        .as_str()
        .map(hash_password)
        .ok_or(IntrinsicError::InvalidArguments("md5"))
}

/// Extracts regex-intrinsic arguments when every value is a string.
fn strings<const N: usize>(values: [&Value; N]) -> Option<[&str; N]> {
    let mut texts = [""; N];

    for (text, value) in texts.iter_mut().zip(values) {
        *text = value.as_str()?;
    }

    Some(texts)
}

fn option(option: &str, intrinsic: &'static str) -> Result<RegexOptions, IntrinsicError> {
    parse_options(option).ok_or(IntrinsicError::InvalidRegexOption(intrinsic))
}

fn regex_error(error: RegexError, intrinsic: &'static str) -> IntrinsicError {
    match error {
        RegexError::Timeout => IntrinsicError::RegexTimeout(intrinsic),
        RegexError::Failure(message) => IntrinsicError::InvalidPattern(message),
    }
}

/// Splits by regex, retaining capture groups; an empty pattern splits into characters.
/// Returns `null` unless text, pattern, and option are strings (`BasicIntrinsics.split`).
#[native(method)]
pub fn split(
    vm: &mut Vm,
    #[host] regex: HostRegexEngine,
    this: &Value,
    pattern: &Value,
    #[default("none")]
    #[name = "regexOptions"]
    options: &Value,
) -> Result<Value, IntrinsicError> {
    const NAME: &str = "split";
    let Some([text, pattern, options]) = strings([this, pattern, options]) else {
        return Ok(Value::Null);
    };

    let option = option(options, NAME)?;

    if pattern.is_empty() {
        let Value::String(subject) = this else {
            return Ok(Value::Null);
        };
        let heap = vm.heap();
        let units = EncodedText::new(subject, heap.string_encoding());
        let characters: Vec<Value> = (0..)
            .map_while(|at| units.get(at))
            .map(Value::from)
            .collect();

        return Ok(Value::List(vm.heap_mut().add_list(characters)));
    }

    if pattern == DOT {
        vm.stdout().print(DOT_WARNING, PrintOptions::default());
    }

    let parts: Vec<Value> = regex
        .split(text, pattern, option)
        .map_err(|error| regex_error(error, NAME))?
        .into_iter()
        .map(Value::from)
        .collect();

    Ok(Value::List(vm.heap_mut().add_list(parts)))
}

/// Replaces every regex match; implements the string `replace` method.
#[native(method)]
pub fn replace_regex(
    #[host] regex: HostRegexEngine,
    this: &Value,
    pattern: &Value,
    newval: &Value,
    #[default("none")]
    #[name = "regexOptions"]
    options: &Value,
) -> Result<Value, IntrinsicError> {
    const NAME: &str = "replace";
    let Some([text, pattern, replacement, options]) = strings([this, pattern, newval, options])
    else {
        return Err(IntrinsicError::RequiresString(NAME));
    };

    if pattern.is_empty() {
        return Err(IntrinsicError::EmptyPattern(NAME));
    }

    let option = option(options, NAME)?;

    if text.is_empty() {
        return Ok(this.clone());
    }

    let replaced_text = regex
        .replace(text, pattern, replacement, option)
        .map_err(|error| regex_error(error, NAME))?;

    Ok(Value::from(replaced_text))
}

/// Tests a regex match; invalid patterns return no match.
#[native(method)]
pub fn is_match(
    #[host] regex: HostRegexEngine,
    this: &Value,
    pattern: &Value,
    #[default("none")]
    #[name = "regexOptions"]
    options: &Value,
) -> Result<bool, IntrinsicError> {
    const NAME: &str = "is_match";
    let Some([text, pattern, options]) = strings([this, pattern, options]) else {
        return Err(IntrinsicError::RequiresString(NAME));
    };

    if pattern.is_empty() {
        return Err(IntrinsicError::EmptyPattern(NAME));
    }

    let option = option(options, NAME)?;

    match regex.is_match(text, pattern, option) {
        Ok(is_match) => Ok(is_match),
        Err(RegexError::Timeout) => Err(IntrinsicError::RegexTimeout(NAME)),
        Err(RegexError::Failure(_)) => Ok(false),
    }
}

/// Returns all regex matches keyed by start offset.
#[native(method)]
pub fn matches(
    vm: &mut Vm,
    #[host] regex: HostRegexEngine,
    this: &Value,
    pattern: &Value,
    #[default("none")]
    #[name = "regexOptions"]
    options: &Value,
) -> Result<Value, IntrinsicError> {
    const NAME: &str = "matches";
    let Some([text, pattern, options]) = strings([this, pattern, options]) else {
        return Err(IntrinsicError::RequiresString(NAME));
    };

    if pattern.is_empty() {
        return Err(IntrinsicError::EmptyPattern("is_match"));
    }

    let option = option(options, NAME)?;
    let entries: Vec<(Value, Value)> = regex
        .matches(text, pattern, option)
        .map_err(|error| regex_error(error, NAME))?
        .into_iter()
        .map(|hit| (Value::from(hit.index as f64), Value::from(hit.value)))
        .collect();

    Ok(Value::Map(vm.heap_mut().add_map_from(entries)))
}

#[native(method)]
pub fn trim(this: &Value) -> Option<&str> {
    this.as_str().map(str::trim)
}

/// Returns the last start offset of `searchStr`, or -1.
/// Empty `searchStr` returns the final position, matching .NET Framework `LastIndexOf`.
#[native(method)]
pub fn last_index_of(
    vm: &mut Vm,
    this: &Value,
    #[name = "searchStr"] search: &Value,
) -> Option<f64> {
    let (Value::String(subject), Some(search)) = (this, search.as_str()) else {
        return None;
    };
    let units = vm.heap().text_units(subject);
    let match_index = if search.is_empty() {
        Some(units.len().saturating_sub(1))
    } else {
        subject.rfind(search).map(|at| units.units_before(at))
    };

    Some(match_index.map_or(-1.0, |at| at as f64))
}

/// Parses a string using .NET `NumberStyles.Any`, allowing decimals, exponents, thousands separators, currency signs, and parentheses.
/// Returns the original string when parsing fails and returns numbers unchanged (`BasicIntrinsics.to_int`).
#[native(method)]
pub fn to_int(#[default(0)] this: &Value) -> Value {
    match this {
        Value::Number(_) => this.clone(),
        Value::String(text) => match mono_number::parse_f64(text, NumberStyles::ANY) {
            Some(number) => Value::from(number),
            None => this.clone(),
        },
        _ => Value::Null,
    }
}
