use std::ops::{Add, Div, Sub};

use text_mesh_core::Alignment;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
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

impl Div<f32> for Point {
    type Output = Self;

    fn div(self, divisor: f32) -> Self {
        Self::new(self.x / divisor, self.y / divisor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EffectTransform {
    m00: f32,
    m01: f32,
    m10: f32,
    m11: f32,
    pub lossy_scale_x: f32,
}

impl EffectTransform {
    pub(crate) fn new(rotation: f32, scale: f32) -> Option<Self> {
        if rotation != 0.0 {
            let half = rotation.to_radians() * 0.5;
            let (z, w) = half.sin_cos();
            let z2 = z + z;
            let zz = z * z2;
            let wz = w * z2;

            return Some(Self {
                m00: 1.0 - zz,
                m01: -wz,
                m10: wz,
                m11: 1.0 - zz,
                lossy_scale_x: 1.0,
            });
        }

        (scale != 1.0).then_some(Self {
            m00: scale,
            m01: 0.0,
            m10: 0.0,
            m11: 1.0,
            lossy_scale_x: scale,
        })
    }

    pub(crate) fn apply(&self, point: Point) -> Point {
        Point::new(
            self.m00 * point.x + self.m01 * point.y,
            self.m10 * point.x + self.m11 * point.y,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct CharacterInfo {
    pub character: u32,
    pub visible: bool,
    pub line: usize,
    pub origin: f32,
    pub x_advance: f32,
    pub baseline: f32,
    pub ascender: f32,
    pub descender: f32,
    pub adjusted_ascender: f32,
    pub adjusted_descender: f32,
    pub top_left: Point,
    pub bottom_left: Point,
    pub top_right: Point,
    pub bottom_right: Point,
    pub point_size: f32,
    pub scale: f32,
    pub font: usize,
    pub padding: f32,
    pub style_padding: f32,
}

impl CharacterInfo {
    pub(crate) fn shift_vertically(&mut self, offset: f32) {
        let shift = Point::new(0.0, offset);

        self.bottom_left = self.bottom_left - shift;
        self.top_left = self.top_left - shift;
        self.top_right = self.top_right - shift;
        self.bottom_right = self.bottom_right - shift;
        self.ascender -= offset;
        self.baseline -= offset;
        self.descender -= offset;
    }

    pub(crate) fn translate(&mut self, offset: Point) {
        self.bottom_left = self.bottom_left + offset;
        self.top_left = self.top_left + offset;
        self.top_right = self.top_right + offset;
        self.bottom_right = self.bottom_right + offset;
        self.origin += offset.x;
        self.x_advance += offset.x;
        self.ascender += offset.y;
        self.descender += offset.y;
        self.baseline += offset.y;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineInfo {
    pub first: usize,
    pub first_visible: usize,
    pub last: usize,
    pub last_visible: usize,
    pub character_count: usize,
    pub visible_character_count: usize,
    pub whitespace_count: i32,
    pub control_character_count: i32,
    pub ascender: f32,
    pub descender: f32,
    pub baseline: f32,
    pub line_height: f32,
    pub width: f32,
    pub length: f32,
    pub max_advance: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub alignment: Alignment,
}

impl LineInfo {
    pub(crate) const fn new(alignment: Alignment) -> Self {
        Self {
            first: 0,
            first_visible: 0,
            last: 0,
            last_visible: 0,
            character_count: 0,
            visible_character_count: 0,
            whitespace_count: 0,
            control_character_count: 0,
            ascender: TextLimits::LARGE_NEGATIVE,
            descender: TextLimits::LARGE_POSITIVE,
            baseline: 0.0,
            line_height: 0.0,
            width: 0.0,
            length: 0.0,
            max_advance: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
            alignment,
        }
    }

    pub(crate) fn translate(&mut self, offset: Point) {
        self.baseline += offset.y;
        self.ascender += offset.y;
        self.descender += offset.y;
        self.max_advance += offset.x;
    }
}

pub(crate) struct TextLimits;

impl TextLimits {
    pub(crate) const LARGE_NEGATIVE: f32 = -32767.0;
    pub(crate) const LARGE_POSITIVE: f32 = 32767.0;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Extents {
    pub min: Point,
    pub max: Point,
}

impl Extents {
    pub(crate) const EMPTY: Self = Self {
        min: Point::new(2_147_483_647.0, 2_147_483_647.0),
        max: Point::new(-2_147_483_647.0, -2_147_483_647.0),
    };

    pub(crate) fn include(&mut self, bottom_left: Point, top_right: Point) {
        self.min.x = self.min.x.min(bottom_left.x);
        self.min.y = self.min.y.min(bottom_left.y);
        self.max.x = self.max.x.max(top_right.x);
        self.max.y = self.max.y.max(top_right.y);
    }
}

#[cfg(test)]
#[path = "tests/records.rs"]
mod tests;
