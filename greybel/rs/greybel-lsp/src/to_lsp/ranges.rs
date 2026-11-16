use gen_lsp_types as lsp;
use greybel_core::Range;
use greybel_type_analyzer::{ColorInformation, Fold, FoldKind, InlayHint, InlayHintKind};

use super::{position, range};

#[must_use]
pub fn fold(fold: &Fold) -> lsp::FoldingRange {
    lsp::FoldingRange {
        start_line: fold.start_line.saturating_sub(1),
        start_character: None,
        end_line: fold.end_line.saturating_sub(1),
        end_character: None,
        kind: match fold.kind {
            FoldKind::Block => None,
            FoldKind::Comment => Some(lsp::FoldingRangeKind::Comment),
            FoldKind::Imports => Some(lsp::FoldingRangeKind::Imports),
        },
        collapsed_text: None,
    }
}

/// Links `ranges`, innermost first, into one selection range whose parents are the outer ranges.
#[must_use]
pub fn selection_range(ranges: &[Range], fallback: lsp::Position) -> lsp::SelectionRange {
    let mut selection = None;

    for &outer in ranges.iter().rev() {
        selection = Some(lsp::SelectionRange {
            range: range(outer),
            parent: selection.map(Box::new),
        });
    }

    selection.unwrap_or(lsp::SelectionRange {
        range: lsp::Range {
            start: fallback,
            end: fallback,
        },
        parent: None,
    })
}

#[must_use]
pub fn inlay_hint(hint: &InlayHint) -> lsp::InlayHint {
    let parameter = hint.kind == InlayHintKind::Parameter;

    lsp::InlayHint {
        position: position(hint.position),
        label: lsp::Label::String(hint.label.clone()),
        kind: Some(if parameter {
            lsp::InlayHintKind::Parameter
        } else {
            lsp::InlayHintKind::Type
        }),
        text_edits: None,
        tooltip: None,
        padding_left: None,
        padding_right: Some(parameter),
        data: None,
    }
}

#[must_use]
pub fn color_information(color: &ColorInformation) -> lsp::ColorInformation {
    let channel = |value: u8| f32::from(value) / 255.0;

    lsp::ColorInformation {
        range: range(color.range),
        color: lsp::Color {
            red: channel(color.red),
            green: channel(color.green),
            blue: channel(color.blue),
            alpha: channel(color.alpha),
        },
    }
}

/// The hex value for a color the client picked.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn color_presentation(color: lsp::Color, value: lsp::Range) -> lsp::ColorPresentation {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let label = ColorInformation::hex(
        channel(color.red),
        channel(color.green),
        channel(color.blue),
        channel(color.alpha),
    );

    lsp::ColorPresentation {
        text_edit: Some(lsp::TextEdit {
            range: value,
            new_text: label.clone(),
        }),
        label,
        additional_text_edits: None,
    }
}
