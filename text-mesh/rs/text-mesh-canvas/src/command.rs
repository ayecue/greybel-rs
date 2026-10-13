use text_mesh_core::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Transform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    #[must_use]
    pub fn then(self, after: Self) -> Self {
        Self {
            a: after.a * self.a + after.c * self.b,
            b: after.b * self.a + after.d * self.b,
            c: after.a * self.c + after.c * self.d,
            d: after.b * self.c + after.d * self.d,
            e: after.a * self.e + after.c * self.f + after.e,
            f: after.b * self.e + after.d * self.f + after.f,
        }
    }

    #[must_use]
    pub const fn translation(x: f32, y: f32) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }

    #[must_use]
    pub fn shear(shear: f32, baseline: f32) -> Self {
        Self {
            c: -shear,
            e: shear * baseline,
            ..Self::IDENTITY
        }
    }

    #[must_use]
    pub fn about(center_x: f32, center_y: f32, degrees: f32, scale: f32) -> Self {
        let (sin, cos) = (-degrees).to_radians().sin_cos();

        Self::translation(-center_x, -center_y)
            .then(Self {
                a: scale,
                ..Self::IDENTITY
            })
            .then(Self {
                a: cos,
                b: sin,
                c: -sin,
                d: cos,
                ..Self::IDENTITY
            })
            .then(Self::translation(center_x, center_y))
    }

    #[must_use]
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Paint {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color> for Paint {
    fn from(Color { r, g, b, a }: Color) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub paint: Paint,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Glyph {
    pub character: char,
    pub font: String,
    pub size: f32,
    pub bold: bool,
    pub x: f32,
    pub baseline: f32,
    pub paint: Paint,
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "lowercase"))]
pub enum DrawCommand {
    Rect(Rect),
    Glyph(Glyph),
}
