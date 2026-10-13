use super::DistanceScale;

/// Measures Unity SDFAA distances with Unity's corner and diagonal handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AntiAliasedDistance {
    width: usize,
    height: usize,
    spread: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Gradient {
    x: f64,
    y: f64,
}

struct NearestEdge<'a> {
    image: &'a [f64],
    width: usize,
    height: usize,
    reach: i64,
}

impl Gradient {
    const DIAGONAL: f64 = std::f64::consts::SQRT_2;

    fn of(image: &[f64], width: usize, height: usize, index: usize) -> Self {
        let (row, column) = (index / width, index % width);

        if row == 0 || column == 0 || row + 1 >= height || column + 1 >= width {
            return Self::default();
        }

        let at = |offset_row: usize, offset_column: usize| {
            image[(row + offset_row - 1) * width + column + offset_column - 1]
        };
        let x = -at(0, 0) - Self::DIAGONAL * at(1, 0) - at(2, 0)
            + at(0, 2)
            + Self::DIAGONAL * at(1, 2)
            + at(2, 2);
        let y = -at(0, 0) - Self::DIAGONAL * at(0, 1) - at(0, 2)
            + at(2, 0)
            + Self::DIAGONAL * at(2, 1)
            + at(2, 2);
        let length = (x * x + y * y).sqrt();

        if length > 0.0 {
            Self {
                x: x / length,
                y: y / length,
            }
        } else {
            Self { x, y }
        }
    }

    fn edge_distance(self, coverage: f64) -> f64 {
        if self.x == 0.0 || self.y == 0.0 {
            return 0.5 - coverage;
        }

        let length = self.x.hypot(self.y);
        let (mut x, mut y) = ((self.x / length).abs(), (self.y / length).abs());

        if x < y {
            std::mem::swap(&mut x, &mut y);
        }

        let corner = 0.5 * y / x;

        if coverage < corner {
            0.5 * (x + y) - (2.0 * x * y * coverage).sqrt()
        } else if coverage < 1.0 - corner {
            (0.5 - coverage) * x
        } else {
            -0.5 * (x + y) + (2.0 * x * y * (1.0 - coverage)).sqrt()
        }
    }
}

impl<'a> NearestEdge<'a> {
    const UNSET: f64 = 1_000_000.0;

    const fn new(image: &'a [f64], width: usize, height: usize, spread: i64) -> Self {
        Self {
            image,
            width,
            height,
            reach: spread + 2,
        }
    }

    fn distances(&self) -> Vec<f64> {
        (0..self.image.len())
            .map(|index| {
                let coverage = self.image[index];

                if coverage >= 1.0 {
                    0.0
                } else if coverage > 0.0 {
                    Gradient::of(self.image, self.width, self.height, index).edge_distance(coverage)
                } else {
                    self.nearest(index)
                }
            })
            .collect()
    }

    fn nearest(&self, index: usize) -> f64 {
        let (row, column) = ((index / self.width) as i64, (index % self.width) as i64);
        let mut best: Option<(i64, f64)> = None;

        for y in -self.reach..=self.reach {
            let neighbor_row = row + y;

            if neighbor_row < 0 || neighbor_row >= self.height as i64 {
                continue;
            }

            for x in -self.reach..=self.reach {
                let neighbor_column = column + x;

                if neighbor_column < 0 || neighbor_column >= self.width as i64 {
                    continue;
                }

                let coverage =
                    self.image[neighbor_row as usize * self.width + neighbor_column as usize];

                if coverage <= 0.0 {
                    continue;
                }

                let squared_distance = x * x + y * y;

                if best.is_some_and(|(nearest, _)| squared_distance > nearest) {
                    continue;
                }

                let distance = (squared_distance as f64).sqrt()
                    + Gradient {
                        x: x as f64,
                        y: y as f64,
                    }
                    .edge_distance(coverage.min(1.0));

                best = match best {
                    Some((nearest, best_distance)) if squared_distance == nearest => {
                        Some((nearest, best_distance.min(distance)))
                    }
                    _ => Some((squared_distance, distance)),
                };
            }
        }

        best.map_or(Self::UNSET, |(_, distance)| distance)
    }
}

impl AntiAliasedDistance {
    #[must_use]
    pub const fn new(width: usize, height: usize, spread: u16) -> Self {
        Self {
            width,
            height,
            spread: spread as i64,
        }
    }

    /// Alpha values for `coverage` (`width` by `height`, rows from the
    /// top).
    #[must_use]
    pub fn alpha(&self, coverage: &[u8]) -> Vec<u8> {
        let image: Vec<f64> = coverage
            .iter()
            .map(|&value| f64::from(value) / 255.0)
            .collect();
        let inverse_image: Vec<f64> = image.iter().map(|value| 1.0 - value).collect();
        let outside = NearestEdge::new(&image, self.width, self.height, self.spread).distances();
        let inside =
            NearestEdge::new(&inverse_image, self.width, self.height, self.spread).distances();
        let scale = DistanceScale::new(self.spread);

        outside
            .iter()
            .zip(&inside)
            .map(|(outside, inside)| scale.alpha(inside.max(0.0) - outside.max(0.0)))
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/anti_aliased_distance.rs"]
mod tests;
