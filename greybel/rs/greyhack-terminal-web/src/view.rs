use greyhack_text_mesh_assets::{TERMINAL_TEXT, TERMINAL_THEME, TEXT_ASSETS};
use text_mesh_core::TerminalOutput;
use text_mesh_layout::{Layout, LayoutGeometry, TerminalLayout, TextLayout};
use text_mesh_webgl::{Mesh, MeshBuilder, SpriteSheet};

/// A laid-out terminal line with its vertical item offset.
#[derive(Debug, Clone)]
struct Line {
    text: String,
    layout: Layout,
    geometry: LayoutGeometry,
    top: f32,
}

/// Scrollable Grey Hack output using `PrototypeTMP`-style item layout. It follows the end until scrolled upward
/// and retains unchanged line layouts as output grows.
#[derive(Debug, Clone)]
pub struct TerminalView {
    width: f32,
    height: f32,
    scroll: f32,
    following: bool,
    lines: Vec<Line>,
    sprites: SpriteSheet,
}

impl TerminalView {
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            scroll: 0.0,
            following: true,
            lines: Vec::new(),
            sprites: SpriteSheet::from_asset(&TERMINAL_TEXT.font_set.sprites).unwrap_or_default(),
        }
    }

    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> f32 {
        self.height
    }

    /// Viewport's vertical offset from the start of the output.
    #[must_use]
    pub const fn scroll(&self) -> f32 {
        self.scroll
    }

    #[must_use]
    pub const fn is_following(&self) -> bool {
        self.following
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns the laid-out output height plus frame padding.
    #[must_use]
    pub fn content_height(&self) -> f32 {
        let padding = TERMINAL_TEXT.frame.padding;

        self.lines.last().map_or(2.0 * padding, |line| {
            line.top + line.layout.height + padding
        })
    }

    pub fn set_text(&mut self, text: &str) {
        let terminal = Self::terminal();
        let frame = terminal.frame();
        let mut previous = std::mem::take(&mut self.lines).into_iter();
        let mut top = frame.padding;

        for source in TerminalOutput::new(text).lines() {
            let line = match previous.next() {
                Some(line) if line.text == source => Line { top, ..line },
                _ => {
                    let (layout, geometry) = terminal.layout_line(&source, self.width);

                    Line {
                        text: source,
                        layout,
                        geometry,
                        top,
                    }
                }
            };

            top += line.layout.height + frame.item_gap;
            self.lines.push(line);
        }

        self.clamp_scroll();
    }

    /// Sets the viewport size and relayouts all lines when its width changes.
    #[expect(
        clippy::float_cmp,
        reason = "An exact width comparison avoids relayout only when the viewport value is unchanged"
    )]
    pub fn resize(&mut self, width: f32, height: f32) {
        self.height = height;
        if width != self.width {
            self.width = width;

            let text: Vec<String> = self.lines.iter().map(|line| line.text.clone()).collect();

            self.lines.clear();
            self.set_text(&text.join("\n"));
        }

        self.clamp_scroll();
    }

    /// Scrolls by `delta` pixels within the output bounds. Positive values move down; following is enabled at the end.
    pub fn scroll_by(&mut self, delta: f32) {
        let end = self.scroll_end();

        self.scroll = (self.scroll + delta).clamp(0.0, end);
        self.following = self.scroll >= end;
    }

    /// Builds meshes for visible lines and a one-viewport-height margin for overflowing glyphs, relative to the
    /// current view.
    #[must_use]
    pub fn meshes(&self) -> Vec<Mesh> {
        let builder = MeshBuilder::new(&TERMINAL_TEXT.font_set, &self.sprites, TERMINAL_THEME.text);
        let padding = TERMINAL_TEXT.frame.padding;
        let (from, to) = (self.scroll - self.height, self.scroll + 2.0 * self.height);

        self.lines
            .iter()
            .filter(|line| line.top + line.layout.height > from && line.top < to)
            .flat_map(|line| {
                builder.build(
                    &line.layout,
                    &line.geometry,
                    [padding, line.top - self.scroll],
                )
            })
            .collect()
    }

    fn terminal() -> TerminalLayout<'static> {
        TerminalLayout::new(
            TextLayout::new(
                &TERMINAL_TEXT.font_set,
                &TEXT_ASSETS,
                &TERMINAL_TEXT.settings,
            ),
            TERMINAL_THEME.component(&TEXT_ASSETS),
            TERMINAL_TEXT.frame,
        )
    }

    fn scroll_end(&self) -> f32 {
        (self.content_height() - self.height).max(0.0)
    }

    fn clamp_scroll(&mut self) {
        let end = self.scroll_end();

        self.scroll = if self.following {
            end
        } else {
            self.scroll.clamp(0.0, end)
        };
        self.following = self.scroll >= end;
    }
}
