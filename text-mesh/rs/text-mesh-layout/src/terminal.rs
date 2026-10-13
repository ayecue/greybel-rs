use text_mesh_core::{TerminalOutput, TextComponent};

use crate::layout::{Layout, LayoutGeometry};
use crate::text_layout::TextLayout;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalFrame {
    pub padding: f32,
    pub item_gap: f32,
    pub item_min_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalItem {
    pub top: f32,
    pub layout: Layout,
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalLayout<'a> {
    text: TextLayout<'a>,
    component: TextComponent<'a>,
    frame: TerminalFrame,
}

impl<'a> TerminalLayout<'a> {
    #[must_use]
    pub const fn new(
        text: TextLayout<'a>,
        component: TextComponent<'a>,
        frame: TerminalFrame,
    ) -> Self {
        Self {
            text,
            component,
            frame,
        }
    }

    #[must_use]
    pub fn layout(&self, output: &str, width: f32) -> Vec<TerminalItem> {
        self.layout_with_geometry(output, width)
            .into_iter()
            .map(|(item, _)| item)
            .collect()
    }

    /// Returns [`Self::layout`] output with [`LayoutGeometry`] for every item.
    #[must_use]
    pub fn layout_with_geometry(
        &self,
        output: &str,
        width: f32,
    ) -> Vec<(TerminalItem, LayoutGeometry)> {
        let mut top = self.frame.padding;
        let mut items = Vec::new();

        for line in TerminalOutput::new(output).lines() {
            let (layout, geometry) = self.layout_line(&line, width);
            let height = layout.height;

            items.push((TerminalItem { top, layout }, geometry));
            top += height + self.frame.item_gap;
        }

        items
    }

    /// Lays out one terminal line at `width`, returning geometry for callers that retain items between changes.
    #[must_use]
    pub fn layout_line(&self, line: &str, width: f32) -> (Layout, LayoutGeometry) {
        let text = self.component.parse(line);

        self.text.layout_with_min_height(
            &text,
            width - 2.0 * self.frame.padding,
            self.frame.item_min_height,
        )
    }

    #[must_use]
    pub const fn frame(&self) -> TerminalFrame {
        self.frame
    }

    #[must_use]
    pub fn height(&self, items: &[TerminalItem]) -> f32 {
        items.last().map_or(2.0 * self.frame.padding, |item| {
            item.top + item.layout.height + self.frame.padding
        })
    }
}
