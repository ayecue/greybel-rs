use super::DistanceScale;
use super::envelope::LowerEnvelope;
use crate::raster::MonochromeRaster;

/// Measures Unity SDF8/16/32 distances on an `upsampling`-scaled bitmap by searching `spread` output pixels in both directions.
#[derive(Debug, Clone, Copy)]
pub struct SampledDistance<'a> {
    raster: &'a MonochromeRaster,
    upsampling: i64,
    spread: i64,
}

impl<'a> SampledDistance<'a> {
    #[must_use]
    pub fn new(raster: &'a MonochromeRaster, upsampling: u16, spread: u16) -> Self {
        Self {
            raster,
            upsampling: i64::from(upsampling.max(1)),
            spread: i64::from(spread),
        }
    }

    /// `width` by `height` alpha values, rows from the top.
    #[must_use]
    pub fn alpha(&self, width: usize, height: usize) -> Vec<u8> {
        let scale = DistanceScale::new(self.spread);
        let columns = self.raster.width();
        let mut alpha = vec![0u8; width * height];
        let mut vertical = [vec![None; columns], vec![None; columns]];

        for row in 0..height {
            let center = (2 * row as i64 + 1) * self.upsampling;
            let (first, last) = self.window(row, self.raster.height());

            for (state, values) in vertical.iter_mut().enumerate() {
                for (column, value) in values.iter_mut().enumerate() {
                    *value = self
                        .raster
                        .nearest_row(column, center, first, last, state == 1)
                        .map(|distance| distance * distance);
                }
            }

            let envelopes = [
                LowerEnvelope::new(&vertical[0]),
                LowerEnvelope::new(&vertical[1]),
            ];
            let sample_row = row * self.upsampling as usize + self.upsampling as usize / 2;

            for column in 0..width {
                let sample_column =
                    column * self.upsampling as usize + self.upsampling as usize / 2;
                let inside = self.raster.is_set(sample_column, sample_row);
                let opposite_state = usize::from(!inside);
                let distance = self.nearest(
                    column,
                    &envelopes[opposite_state],
                    &vertical[opposite_state],
                );

                alpha[row * width + column] = match distance {
                    Some(squared_distance) => {
                        let pixels =
                            (squared_distance as f64).sqrt() / 2.0 / self.upsampling as f64;

                        scale.alpha(if inside { pixels } else { -pixels })
                    }
                    None => scale.far_alpha(inside),
                };
            }
        }

        alpha
    }

    fn window(&self, index: usize, len: usize) -> (i64, i64) {
        let index = index as i64;
        let first = ((index - self.spread) * self.upsampling).max(0);
        let last = ((index + self.spread) * self.upsampling).min(len as i64 - 1);

        (first, last)
    }

    /// Computes twice the squared distance from the center of `column`'s block to the nearest searched-state pixel in the window.
    fn nearest(
        &self,
        column: usize,
        envelope: &LowerEnvelope,
        vertical: &[Option<i64>],
    ) -> Option<i64> {
        let center = (2 * column as i64 + 1) * self.upsampling;
        let (first, last) = self.window(column, vertical.len());
        let (squared_distance, nearest) = envelope.query(center)?;

        if (first..=last).contains(&(nearest as i64)) {
            return Some(squared_distance);
        }

        (first.max(0)..=last)
            .filter_map(|opposite_state| {
                let offset = 2 * opposite_state + 1 - center;

                vertical[opposite_state as usize].map(|value| offset * offset + value)
            })
            .min()
    }
}

#[cfg(test)]
#[path = "tests/sampled_distance.rs"]
mod tests;
