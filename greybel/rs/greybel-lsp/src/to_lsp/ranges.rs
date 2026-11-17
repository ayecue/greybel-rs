use gen_lsp_types as lsp;
use greybel_core::Range;
use greybel_type_analyzer::{ColorInformation, Fold, FoldKind, InlayHint, InlayHintKind};

use super::ToLsp;

impl ToLsp for Fold {
    type Output = lsp::FoldingRange;

    fn to_lsp(&self) -> lsp::FoldingRange {
        lsp::FoldingRange {
            start_line: self.start_line.saturating_sub(1),
            start_character: None,
            end_line: self.end_line.saturating_sub(1),
            end_character: None,
            kind: self.kind.to_lsp(),
            collapsed_text: None,
        }
    }
}

impl ToLsp for FoldKind {
    type Output = Option<lsp::FoldingRangeKind>;

    fn to_lsp(&self) -> Option<lsp::FoldingRangeKind> {
        match self {
            FoldKind::Block => None,
            FoldKind::Comment => Some(lsp::FoldingRangeKind::Comment),
            FoldKind::Imports => Some(lsp::FoldingRangeKind::Imports),
        }
    }
}

pub(crate) struct SelectionRange<'a> {
    pub ranges: &'a [Range],
    pub fallback: lsp::Position,
}

impl ToLsp for SelectionRange<'_> {
    type Output = lsp::SelectionRange;

    fn to_lsp(&self) -> lsp::SelectionRange {
        let mut selection = None;

        for outer in self.ranges.iter().rev() {
            selection = Some(lsp::SelectionRange {
                range: outer.to_lsp(),
                parent: selection.map(Box::new),
            });
        }

        selection.unwrap_or(lsp::SelectionRange {
            range: lsp::Range {
                start: self.fallback,
                end: self.fallback,
            },
            parent: None,
        })
    }
}

impl ToLsp for InlayHint {
    type Output = lsp::InlayHint;

    fn to_lsp(&self) -> lsp::InlayHint {
        lsp::InlayHint {
            position: self.position.to_lsp(),
            label: lsp::Label::String(self.label.clone()),
            kind: Some(self.kind.to_lsp()),
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: Some(self.kind == InlayHintKind::Parameter),
            data: None,
        }
    }
}

impl ToLsp for InlayHintKind {
    type Output = lsp::InlayHintKind;

    fn to_lsp(&self) -> lsp::InlayHintKind {
        match self {
            InlayHintKind::Parameter => lsp::InlayHintKind::Parameter,
            InlayHintKind::Type => lsp::InlayHintKind::Type,
        }
    }
}

impl ToLsp for ColorInformation {
    type Output = lsp::ColorInformation;

    fn to_lsp(&self) -> lsp::ColorInformation {
        let channel = |value: u8| f32::from(value) / 255.0;

        lsp::ColorInformation {
            range: self.range.to_lsp(),
            color: lsp::Color {
                red: channel(self.red),
                green: channel(self.green),
                blue: channel(self.blue),
                alpha: channel(self.alpha),
            },
        }
    }
}

pub(crate) struct ColorPresentation {
    pub color: lsp::Color,
    pub range: lsp::Range,
}

impl ToLsp for ColorPresentation {
    type Output = lsp::ColorPresentation;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn to_lsp(&self) -> lsp::ColorPresentation {
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        let color = self.color;
        let label = ColorInformation::hex(
            channel(color.red),
            channel(color.green),
            channel(color.blue),
            channel(color.alpha),
        );

        lsp::ColorPresentation {
            text_edit: Some(lsp::TextEdit {
                range: self.range,
                new_text: label.clone(),
            }),
            label,
            additional_text_edits: None,
        }
    }
}
