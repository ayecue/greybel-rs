/// Creates a Unity-canvas-style orthographic projection for a y-down `width` × `height` pixel frame at `pixel_ratio` device pixels per pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    pub width: f32,
    pub height: f32,
    pub pixel_ratio: f32,
}

impl Projection {
    /// Stores a column-major transform mapping x from 0..width to -1..1 and y from 0..height to 1..-1.
    #[must_use]
    pub fn matrix(&self) -> [f32; 16] {
        [
            2.0 / self.width,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / self.height,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            -1.0,
            1.0,
            0.0,
            1.0,
        ]
    }

    /// Drawing buffer's size in device pixels (`_ScreenParams.xy`).
    #[must_use]
    pub fn screen(&self) -> [f32; 2] {
        [
            (self.width * self.pixel_ratio).round(),
            (self.height * self.pixel_ratio).round(),
        ]
    }
}
