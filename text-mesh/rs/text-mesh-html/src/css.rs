use std::fmt::Write as _;

use text_mesh_core::{Color, FontWeight, Style, TextComponent};

pub(crate) struct Declarations(String);

impl Declarations {
    const SMALL_CAPS_SCALE: f32 = 0.8;

    pub(crate) fn new(style: &Style, component: &TextComponent) -> Self {
        let mut declarations = Self(String::new());
        let em = |value: f32| value / component.size;

        if style.color != component.color {
            declarations.push("color", &Self::color(style.color));
        }

        if let Some(font) = style
            .font
            .and_then(|font| component.assets.fonts.get(usize::from(font)))
        {
            declarations.push("font-family", &Self::string(font));
        }

        if style.weight != FontWeight::REGULAR {
            declarations.push("font-weight", &style.weight.0.to_string());
        }

        if style.italic {
            declarations.push("font-style", "italic");
        }

        let lines: Vec<&str> = [
            style.underline.map(|_| "underline"),
            style.strikethrough.map(|_| "line-through"),
        ]
        .into_iter()
        .flatten()
        .collect();

        if !lines.is_empty() {
            declarations.push("text-decoration-line", &lines.join(" "));
        }

        if let Some(color) = style.underline.or(style.strikethrough) {
            declarations.push("text-decoration-color", &Self::color(color));
        }

        if let Some(color) = style.highlight {
            declarations.push("background-color", &Self::color(color));
        }

        let scale = if style.small_caps {
            Self::SMALL_CAPS_SCALE
        } else {
            1.0
        };
        let size = style.size * scale;

        if size != component.size {
            declarations.push("font-size", &format!("{}em", Self::number(em(size))));
        }

        let own_em = |value: f32| value / size;

        if style.vertical_offset != 0.0 {
            declarations.push(
                "vertical-align",
                &format!("{}em", Self::number(own_em(style.vertical_offset))),
            );
        } else if style.subscript {
            declarations.push("vertical-align", "sub");
        } else if style.superscript {
            declarations.push("vertical-align", "super");
        }

        if style.character_spacing != 0.0 {
            declarations.push(
                "letter-spacing",
                &format!("{}em", Self::number(own_em(style.character_spacing))),
            );
        }

        declarations
    }

    pub(crate) fn transform(style: &Style) -> Option<(Option<String>, String)> {
        if style.mono_spacing != 0.0 {
            let width = format!(
                "display:inline-block;width:{}em;text-align:center",
                Self::number(style.mono_spacing / style.size)
            );

            return Some(
                match Self::transform(&Style {
                    mono_spacing: 0.0,
                    ..*style
                }) {
                    Some((_, inner)) => (Some(width), inner),
                    None => (Some(width), "display:inline".to_owned()),
                },
            );
        }

        if style.rotation != 0.0 {
            Some((
                None,
                format!(
                    "display:inline-block;transform:rotate({}deg)",
                    Self::number(-style.rotation)
                ),
            ))
        } else if style.scale == 1.0 {
            None
        } else {
            Some((
                Some(format!(
                    "display:inline-block;width:{}ch",
                    Self::number(style.scale)
                )),
                format!(
                    "display:inline-block;transform:scaleX({});transform-origin:left",
                    Self::number(style.scale)
                ),
            ))
        }
    }

    pub(crate) fn sprite(color: Color, style: &Style, component: &TextComponent) -> Self {
        let mut declarations = Self(String::new());

        declarations.push("color", &Self::color(color));
        if style.size != component.size {
            declarations.push("font-size", &Self::em(style.size, component));
        }

        if style.vertical_offset != 0.0 {
            declarations.push(
                "vertical-align",
                &format!("{}em", Self::number(style.vertical_offset / style.size)),
            );
        }

        declarations
    }

    pub(crate) fn em(value: f32, component: &TextComponent) -> String {
        format!("{}em", Self::number(value / component.size))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    fn push(&mut self, property: &str, value: &str) {
        if !self.0.is_empty() {
            self.0.push(';');
        }

        let _ = write!(self.0, "{property}:{value}");
    }

    fn string(text: &str) -> String {
        let mut string_literal = String::with_capacity(text.len() + 2);

        string_literal.push('\'');
        for character in text.chars() {
            match character {
                '\'' | '\\' => {
                    string_literal.push('\\');
                    string_literal.push(character);
                }
                '"' => string_literal.push_str("&quot;"),
                '<' => string_literal.push_str("&lt;"),
                '&' => string_literal.push_str("&amp;"),
                _ => string_literal.push(character),
            }
        }

        string_literal.push('\'');

        string_literal
    }

    pub(crate) fn color(Color { r, g, b, a }: Color) -> String {
        if a == 255 {
            format!("rgb({r},{g},{b})")
        } else {
            format!("rgba({r},{g},{b},{})", Self::number(f32::from(a) / 255.0))
        }
    }

    fn number(value: f32) -> String {
        let decimal_value = (value * 1000.0).round() / 1000.0;

        format!("{decimal_value}")
    }
}
