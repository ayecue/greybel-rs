use crate::error::FontBuildError;

/// Top-down alpha pixels for one atlas texture; glyph rectangles use Unity's bottom-up y coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasImage {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

impl AtlasImage {
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; usize::from(width) * usize::from(height)],
        }
    }

    #[must_use]
    pub fn pixel(&self, x: usize, row: usize) -> u8 {
        if x >= usize::from(self.width) || row >= usize::from(self.height) {
            return 0;
        }

        self.pixels[row * usize::from(self.width) + x]
    }

    /// Blits `pixels` with their bottom-left corner at `x`, `y` in y-up coordinates.
    pub fn draw(&mut self, x: usize, y: usize, width: usize, height: usize, pixels: &[u8]) {
        let atlas_width = usize::from(self.width);
        let atlas_height = usize::from(self.height);

        for row in 0..height {
            let Some(target_row) = (atlas_height + row).checked_sub(y + height) else {
                continue;
            };

            if target_row >= atlas_height {
                continue;
            }

            for column in 0..width.min(atlas_width.saturating_sub(x)) {
                self.pixels[target_row * atlas_width + x + column] = pixels[row * width + column];
            }
        }
    }

    /// Image as an 8-bit grayscale PNG.
    ///
    /// # Errors
    /// When the encoder fails.
    pub fn encode_png(&self) -> Result<Vec<u8>, FontBuildError> {
        let mut bytes = Vec::new();

        {
            let mut encoder =
                png::Encoder::new(&mut bytes, u32::from(self.width), u32::from(self.height));

            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header()?;

            writer.write_image_data(&self.pixels)?;
        }

        Ok(bytes)
    }
}
