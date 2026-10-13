const BOLD: &str = "1";
const ITALIC: &str = "3";
const UNDERLINE: &str = "4";
const STRIKETHROUGH: &str = "9";
const FOREGROUND_RGB: &str = "38;2";
const BACKGROUND_RGB: &str = "48;2";

use std::fmt::Write as _;

use text_mesh_core::{Color, FontWeight, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Sgr {
    foreground: Option<Color>,
    background: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

impl Sgr {
    pub(crate) fn new(style: &Style, default: Color, background: Color, paint: bool) -> Self {
        let foreground = style.color.blend(background);
        let highlight = style.highlight.map(|highlight| highlight.blend(background));

        Self {
            foreground: (paint || foreground != default.blend(background)).then_some(foreground),
            background: highlight.or(paint.then_some(background)),
            bold: style.bold || style.weight >= FontWeight::SEMI_BOLD,
            italic: style.italic,
            underline: style.underline.is_some(),
            strikethrough: style.strikethrough.is_some(),
        }
    }

    pub(crate) fn blank(default: Color, background: Color, paint: bool) -> Self {
        Self {
            foreground: paint.then(|| default.blend(background)),
            background: paint.then_some(background),
            ..Self::default()
        }
    }

    pub(crate) fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    pub(crate) fn write(&self, out: &mut String) {
        let mut codes: Vec<String> = Vec::new();

        if self.bold {
            codes.push(BOLD.to_owned());
        }

        if self.italic {
            codes.push(ITALIC.to_owned());
        }

        if self.underline {
            codes.push(UNDERLINE.to_owned());
        }

        if self.strikethrough {
            codes.push(STRIKETHROUGH.to_owned());
        }

        if let Some(Color { r, g, b, .. }) = self.foreground {
            codes.push(format!("{FOREGROUND_RGB};{r};{g};{b}"));
        }

        if let Some(Color { r, g, b, .. }) = self.background {
            codes.push(format!("{BACKGROUND_RGB};{r};{g};{b}"));
        }

        let _ = write!(out, "\x1b[{}m", codes.join(";"));
    }

    pub(crate) fn reset(out: &mut String) {
        out.push_str("\x1b[0m");
    }
}
