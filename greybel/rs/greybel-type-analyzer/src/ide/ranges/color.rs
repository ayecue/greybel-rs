use greybel_core::{Ast, ColumnSettings, LiteralValue, NodeKind, Range, Span};
use text_mesh_core::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorInformation {
    pub range: Range,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl ColorInformation {
    const TAGS: [&str; 3] = ["<color=", "<mark=", "<#"];

    pub(super) fn collect(ast: &Ast) -> Vec<Self> {
        let mut colors = Vec::new();

        for &literal in &ast.literals {
            let NodeKind::Literal(node) = ast.kind(literal) else {
                continue;
            };

            if matches!(node.value, LiteralValue::String(_)) {
                Self::scan(ast, ast.span(literal), &mut colors);
            }
        }

        colors
    }

    fn scan(ast: &Ast, span: Span, colors: &mut Vec<Self>) {
        let text = ast.slice(span);
        let lowercase = text.to_ascii_lowercase();
        let mut offset = 0;

        while let Some((tag_start, tag)) = Self::TAGS
            .iter()
            .filter_map(|tag| Some((lowercase[offset..].find(tag)? + offset, *tag)))
            .min_by_key(|&(start, _)| start)
        {
            let value_start = tag_start + tag.len() - usize::from(tag == "<#");
            let Some(length) = text[value_start..].find('>') else {
                return;
            };
            let raw = &text[value_start..value_start + length];
            let value = raw.trim_matches('"');
            let quote = raw.len() - raw.trim_start_matches('"').len();

            offset = value_start + length;

            let Some(color) = Color::from_tag_value(value) else {
                continue;
            };
            let start = span.start + u32::try_from(value_start + quote).unwrap_or(0);
            let end = start + u32::try_from(value.len()).unwrap_or(0);

            colors.push(Self {
                range: ast
                    .lines()
                    .range(Span::new(start, end), ColumnSettings::default()),
                red: color.r,
                green: color.g,
                blue: color.b,
                alpha: color.a,
            });
        }
    }

    #[must_use]
    pub fn hex(red: u8, green: u8, blue: u8, alpha: u8) -> String {
        if alpha == u8::MAX {
            format!("#{red:02X}{green:02X}{blue:02X}")
        } else {
            format!("#{red:02X}{green:02X}{blue:02X}{alpha:02X}")
        }
    }
}
