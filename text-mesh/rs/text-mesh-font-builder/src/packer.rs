/// Rectangle in atlas pixels, y up from the texture's bottom, as Unity's `GlyphRect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PackedRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Implements Unity's maximal-rectangles packer with best-short-side-fit placement in input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasPacker {
    free_rects: Vec<PackedRect>,
}

impl PackedRect {
    const fn right(self) -> u32 {
        self.x + self.width
    }

    const fn top(self) -> u32 {
        self.y + self.height
    }

    const fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.top() <= self.top()
    }

    const fn intersects(self, other: Self) -> bool {
        other.x < self.right()
            && other.right() > self.x
            && other.y < self.top()
            && other.top() > self.y
    }
}

impl AtlasPacker {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            free_rects: vec![PackedRect {
                x: 0,
                y: 0,
                width,
                height,
            }],
        }
    }

    /// Places a `width` by `height` rectangle in a fitting free rectangle, or returns `None` when none fits.
    pub fn insert(&mut self, width: u32, height: u32) -> Option<PackedRect> {
        let slot = self.best_short_side_fit(width, height)?;
        let mut index = 0;
        let mut free_count = self.free_rects.len();

        while index < free_count {
            let free_rect = self.free_rects[index];

            if free_rect.intersects(slot) {
                self.split(free_rect, slot);
                self.free_rects.remove(index);
                free_count -= 1;
            } else {
                index += 1;
            }
        }

        self.prune();

        Some(slot)
    }

    fn best_short_side_fit(&self, width: u32, height: u32) -> Option<PackedRect> {
        let mut best: Option<(PackedRect, (u32, u32))> = None;

        for free_rect in &self.free_rects {
            if free_rect.width < width || free_rect.height < height {
                continue;
            }

            let (horizontal, vertical) = (free_rect.width - width, free_rect.height - height);
            let score = (horizontal.min(vertical), horizontal.max(vertical));

            if best.is_none_or(|(_, best)| score < best) {
                let candidate = PackedRect {
                    x: free_rect.x,
                    y: free_rect.y,
                    width,
                    height,
                };

                best = Some((candidate, score));
            }
        }

        best.map(|(slot, _)| slot)
    }

    fn split(&mut self, free_rect: PackedRect, placed_rect: PackedRect) {
        if placed_rect.x < free_rect.right() && placed_rect.right() > free_rect.x {
            if placed_rect.y > free_rect.y && placed_rect.y < free_rect.top() {
                self.free_rects.push(PackedRect {
                    height: placed_rect.y - free_rect.y,
                    ..free_rect
                });
            }

            if placed_rect.top() < free_rect.top() {
                self.free_rects.push(PackedRect {
                    y: placed_rect.top(),
                    height: free_rect.top() - placed_rect.top(),
                    ..free_rect
                });
            }
        }

        if placed_rect.y < free_rect.top() && placed_rect.top() > free_rect.y {
            if placed_rect.x > free_rect.x && placed_rect.x < free_rect.right() {
                self.free_rects.push(PackedRect {
                    width: placed_rect.x - free_rect.x,
                    ..free_rect
                });
            }

            if placed_rect.right() < free_rect.right() {
                self.free_rects.push(PackedRect {
                    x: placed_rect.right(),
                    width: free_rect.right() - placed_rect.right(),
                    ..free_rect
                });
            }
        }
    }

    fn prune(&mut self) {
        let mut first = 0;

        while first < self.free_rects.len() {
            let mut second = first + 1;
            let mut removed = false;

            while second < self.free_rects.len() {
                if self.free_rects[second].contains(self.free_rects[first]) {
                    self.free_rects.remove(first);
                    removed = true;
                    break;
                }

                if self.free_rects[first].contains(self.free_rects[second]) {
                    self.free_rects.remove(second);
                } else {
                    second += 1;
                }
            }

            if !removed {
                first += 1;
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/packer.rs"]
mod tests;
