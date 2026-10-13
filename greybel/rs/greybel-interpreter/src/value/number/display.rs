use std::borrow::Cow;
use std::fmt;
use std::sync::LazyLock;

use mono_number::Format;

use super::Number;

const NEGATIVE_ZERO: &str = "-0";
const ZERO: &str = "0";

const EXACT_INTEGER_LIMIT: f64 = 1e15;
const LARGEST_FIXED_NOTATION: f64 = 1e10;
const SMALLEST_FIXED_NOTATION: f64 = 1e-6;

static INTEGER_FORMAT: LazyLock<Format> = LazyLock::new(|| Format::new("0"));
static SCIENTIFIC: LazyLock<Format> = LazyLock::new(|| Format::new("E6"));
static FIXED_NOTATION: LazyLock<Format> = LazyLock::new(|| Format::new("0.0#####"));

impl Number {
    pub(crate) fn text(self, buffer: &mut itoa::Buffer) -> Cow<'_, str> {
        let value = self.0;

        if value % 1.0 == 0.0 {
            if value.abs() < EXACT_INTEGER_LIMIT {
                #[allow(clippy::cast_possible_truncation)]
                return Cow::Borrowed(buffer.format(value as i64));
            }

            return Cow::Owned(self.format_with(&INTEGER_FORMAT));
        }

        if value.abs() > LARGEST_FIXED_NOTATION || value.abs() < SMALLEST_FIXED_NOTATION {
            return Cow::Owned(self.format_with(&SCIENTIFIC).replace("E-00", "E-0"));
        }

        Cow::Owned(self.format_with(&FIXED_NOTATION))
    }

    fn format_with(self, format: &Format) -> String {
        let text = format
            .f64(self.0)
            .expect("MiniScript's formats apply to doubles");

        if text == NEGATIVE_ZERO {
            ZERO.to_owned()
        } else {
            text
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text(&mut itoa::Buffer::new()))
    }
}
