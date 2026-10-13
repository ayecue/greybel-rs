mod font;
mod literal;
mod sprite;

use std::fmt;

use literal::Literal;

/// Value written as the Rust expression that rebuilds it, with tables borrowed so a generated file can be a `static` initializer.
#[derive(Debug, Clone, Copy)]
pub struct RustSource<'a, T> {
    value: &'a T,
    indent: usize,
}

impl<'a, T> RustSource<'a, T> {
    #[must_use]
    pub const fn new(value: &'a T) -> Self {
        Self { value, indent: 0 }
    }

    const fn descend<U>(&self, value: &'a U) -> RustSource<'a, U> {
        RustSource {
            value,
            indent: self.indent + 1,
        }
    }

    fn open(&self, f: &mut fmt::Formatter<'_>, path: &str) -> fmt::Result {
        writeln!(f, "{path} {{")
    }

    fn field(
        &self,
        f: &mut fmt::Formatter<'_>,
        name: &str,
        value: impl fmt::Display,
    ) -> fmt::Result {
        writeln!(f, "{}{name}: {value},", Self::pad(self.indent + 1))
    }

    fn close(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}}}", Self::pad(self.indent))
    }

    fn slice<U>(&self, f: &mut fmt::Formatter<'_>, name: &str, items: &'a [U]) -> fmt::Result
    where
        RustSource<'a, U>: fmt::Display,
    {
        if items.is_empty() {
            return self.field(f, name, "::std::borrow::Cow::Borrowed(&[])");
        }

        writeln!(
            f,
            "{}{name}: ::std::borrow::Cow::Borrowed(&[",
            Self::pad(self.indent + 1)
        )?;
        for item in items {
            let item = RustSource {
                value: item,
                indent: self.indent + 2,
            };

            writeln!(f, "{}{item},", Self::pad(self.indent + 2))?;
        }

        writeln!(f, "{}]),", Self::pad(self.indent + 1))
    }

    fn pad(indent: usize) -> String {
        "    ".repeat(indent)
    }
}

impl fmt::Display for RustSource<'_, usize> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl fmt::Display for RustSource<'_, f32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Literal::float(*self.value))
    }
}
