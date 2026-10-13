//! Drawing commands from a layout: glyphs, then the decorations TMP draws over them, then sprites.

use text_mesh_canvas::{CanvasPainter, DrawCommand, Glyph, Paint, Rect, Transform};
use text_mesh_core::{Color, Style};

use crate::support::{fonts, glyph, layout, sprite, style};

#[test]
fn should_draw_a_glyph_at_its_pen_position_in_its_font_size() {
    let fonts = fonts();
    let layout = layout(vec![glyph('a', 5.0, style())]);
    let commands = CanvasPainter::new(&fonts).paint(&layout, 10.0, 100.0);

    assert_eq!(
        commands,
        [DrawCommand::Glyph(Glyph {
            character: 'a',
            font: "Mono SDF".to_owned(),
            size: 20.0,
            bold: false,
            x: 15.0,
            baseline: 118.0,
            paint: Paint::from(Color::WHITE),
            transform: None,
        })]
    );
}

#[test]
fn should_not_draw_an_invisible_glyph() {
    let fonts = fonts();
    let layout = layout(vec![glyph(' ', 0.0, style())]);
    let commands = CanvasPainter::new(&fonts).paint(&layout, 0.0, 0.0);

    assert!(commands.is_empty());
}

#[test]
fn should_underline_a_space_across_its_advance() {
    let fonts = fonts();
    let underlined = Style {
        underline: Some(Color::WHITE),
        ..style()
    };
    let layout = layout(vec![glyph(' ', 0.0, underlined)]);
    let commands = CanvasPainter::new(&fonts).paint(&layout, 0.0, 0.0);

    assert_eq!(
        commands,
        [DrawCommand::Rect(Rect {
            x: 0.0,
            y: 19.5,
            width: 12.0,
            height: 1.0,
            paint: Paint::from(Color::WHITE),
        })]
    );
}

#[test]
fn should_draw_decorations_after_glyphs_and_sprites_last() {
    let fonts = fonts();
    let marked = Style {
        highlight: Some(Color::new(255, 255, 0, 64)),
        ..style()
    };
    let layout = layout(vec![
        sprite(0.0),
        glyph('a', 0.0, marked),
        glyph('b', 12.0, style()),
    ]);
    let commands = CanvasPainter::new(&fonts).paint(&layout, 0.0, 0.0);
    let kinds: Vec<&str> = commands
        .iter()
        .map(|command| match command {
            DrawCommand::Glyph(_) => "glyph",
            DrawCommand::Rect(rect) if rect.paint.r == 255 && rect.paint.g == 0 => "sprite",
            DrawCommand::Rect(_) => "highlight",
        })
        .collect();

    assert_eq!(kinds, ["glyph", "glyph", "highlight", "sprite"]);
}

#[test]
fn should_shear_an_italic_glyph_about_its_baseline() {
    let fonts = fonts();
    let italic = Style {
        italic: true,
        ..style()
    };
    let layout = layout(vec![glyph('a', 0.0, italic)]);
    let commands = CanvasPainter::new(&fonts).paint(&layout, 0.0, 0.0);
    let DrawCommand::Glyph(Glyph {
        transform: Some(transform),
        ..
    }) = &commands[0]
    else {
        panic!("{commands:?}");
    };
    let (baseline, above) = (transform.apply(0.0, 18.0), transform.apply(0.0, 8.0));

    assert!(
        baseline == (0.0, 18.0) && (above.0 - 3.5).abs() < 1e-5 && above.1 == 8.0,
        "{baseline:?} {above:?}"
    );
}

#[test]
fn should_turn_a_rotated_glyph_about_the_centre_of_its_quad() {
    let turned = Transform::about(6.0, 12.0, 90.0, 1.0);
    let corner = turned.apply(11.0, 12.0);

    assert!(
        (corner.0 - 6.0).abs() < 1e-5 && (corner.1 - 7.0).abs() < 1e-5,
        "{corner:?}"
    );
}
