use std::ops::{Add, Sub};

use text_mesh_layout::Position;

/// Point in TMP's mesh space (x right, y up); layout positions have y down and convert by negating y.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) const fn from_layout(position: Position) -> Self {
        Self::new(position.x, -position.y)
    }

    pub(crate) fn to_layout(self, offset: [f32; 2]) -> [f32; 2] {
        [self.x + offset[0], -self.y + offset[1]]
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}
