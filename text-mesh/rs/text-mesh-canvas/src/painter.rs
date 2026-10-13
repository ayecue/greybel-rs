use text_mesh_core::Style;
use text_mesh_font::FontSet;
use text_mesh_layout::{Element, ElementKind, Layout};

use crate::command::{DrawCommand, Glyph, Paint, Rect, Transform};

#[derive(Debug, Clone, Copy)]
pub struct CanvasPainter<'a> {
    fonts: &'a FontSet,
}

impl<'a> CanvasPainter<'a> {
    #[must_use]
    pub const fn new(fonts: &'a FontSet) -> Self {
        Self { fonts }
    }

    #[must_use]
    pub fn paint(&self, layout: &Layout, x: f32, y: f32) -> Vec<DrawCommand> {
        let offset = Transform::translation(x, y);
        let mut commands = Vec::new();
        let mut decorations = Vec::new();
        let mut sprites = Vec::new();

        for element in &layout.elements {
            match element.kind {
                ElementKind::Glyph { character, font } => {
                    if element.visible {
                        commands.push(self.glyph(element, character, font, offset));
                    }

                    self.decorate(element, font, offset, &mut decorations);
                }
                ElementKind::Sprite { color, .. } => {
                    sprites.push(Self::rect(
                        offset,
                        element.left,
                        element.top,
                        element.right,
                        element.bottom,
                        color.into(),
                    ));
                }
            }
        }

        commands.extend(decorations);
        commands.extend(sprites);

        commands
    }

    fn glyph(
        &self,
        element: &Element,
        character: char,
        font: usize,
        offset: Transform,
    ) -> DrawCommand {
        let asset = self.fonts.font(font);
        let style = &element.style;
        let mut transform = Transform::IDENTITY;

        if style.italic {
            transform = transform.then(Transform::shear(
                f32::from(asset.style.italic_style) * 0.01,
                element.baseline,
            ));
        }

        if style.rotation != 0.0 || style.scale != 1.0 {
            transform = transform.then(Transform::about(
                (element.left + element.right) / 2.0,
                (element.top + element.bottom) / 2.0,
                style.rotation,
                style.scale,
            ));
        }

        let placed = transform != Transform::IDENTITY;
        let transform = transform.then(offset);
        let (x, baseline) = if placed {
            (element.origin, element.baseline)
        } else {
            transform.apply(element.origin, element.baseline)
        };

        DrawCommand::Glyph(Glyph {
            character,
            font: asset.name.to_string(),
            size: element.scale * asset.face.point_size / asset.face.scale,
            bold: style.bold,
            x,
            baseline,
            paint: style.color.into(),
            transform: placed.then_some(transform),
        })
    }

    fn decorate(
        &self,
        element: &Element,
        font: usize,
        offset: Transform,
        out: &mut Vec<DrawCommand>,
    ) {
        let face = &self.fonts.font(font).face;
        let Style {
            highlight,
            underline,
            strikethrough,
            ..
        } = element.style;

        if let Some(color) = highlight {
            out.push(Self::rect(
                offset,
                element.origin,
                element.ascender,
                element.advance,
                element.descender,
                color.into(),
            ));
        }

        let line = |position: f32, thickness: f32| {
            let middle = element.baseline - position * element.scale;
            let half = thickness * element.scale / 2.0;

            (middle - half, middle + half)
        };

        if let Some(color) = underline {
            let (top, bottom) = line(face.underline_offset, face.underline_thickness);

            out.push(Self::rect(
                offset,
                element.origin,
                top,
                element.advance,
                bottom,
                color.into(),
            ));
        }

        if let Some(color) = strikethrough {
            let (top, bottom) = line(face.strikethrough_offset, face.strikethrough_thickness);

            out.push(Self::rect(
                offset,
                element.origin,
                top,
                element.advance,
                bottom,
                color.into(),
            ));
        }
    }

    fn rect(
        offset: Transform,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        paint: Paint,
    ) -> DrawCommand {
        let (x, y) = offset.apply(left.min(right), top.min(bottom));

        DrawCommand::Rect(Rect {
            x,
            y,
            width: (right - left).abs(),
            height: (bottom - top).abs(),
            paint,
        })
    }
}
