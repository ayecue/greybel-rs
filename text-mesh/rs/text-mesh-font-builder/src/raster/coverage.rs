use super::path::FlatPath;

/// Anti-aliased bitmap with exact per-pixel coverage encoded as 8-bit FreeType-compatible gray values in top-down rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageRaster {
    alpha: Vec<u8>,
}

impl CoverageRaster {
    #[must_use]
    pub fn new(path: &FlatPath, left: f64, top: f64, width: usize, height: usize) -> Self {
        let stride = width + 2;
        let mut area = vec![0.0f64; stride * height];

        for line in path.lines() {
            let from = (line.from.0 - left, top - line.from.1);
            let to = (line.to.0 - left, top - line.to.1);

            Self::accumulate(&mut area, stride, height, from, to);
        }

        let mut alpha = vec![0u8; width * height];

        for row in 0..height {
            let mut sum = 0.0;

            for column in 0..width {
                sum += area[row * stride + column];
                alpha[row * width + column] = (sum.abs() * 256.0).floor().min(255.0) as u8;
            }
        }

        Self { alpha }
    }

    fn accumulate(
        area: &mut [f64],
        stride: usize,
        height: usize,
        from: (f64, f64),
        to: (f64, f64),
    ) {
        if from.1 == to.1 {
            return;
        }

        let (direction, top, bottom) = if from.1 < to.1 {
            (1.0, from, to)
        } else {
            (-1.0, to, from)
        };
        let slope = (bottom.0 - top.0) / (bottom.1 - top.1);
        let first_row = top.1.max(0.0).floor() as usize;
        let last_row = (bottom.1.ceil().max(0.0) as usize).min(height);
        let mut x = top.0 + (top.1.max(0.0) - top.1) * slope;
        let last = stride as i64 - 1;
        let mut add = |row: usize, column: i64, value: f64| {
            let column = column.clamp(0, last) as usize;

            area[row * stride + column] += value;
        };

        for row in first_row..last_row {
            let y0 = (row as f64).max(top.1);
            let y1 = ((row + 1) as f64).min(bottom.1);
            let dy = y1 - y0;
            let x_next = x + slope * dy;
            let height = dy * direction;
            let (x0, x1) = if x < x_next { (x, x_next) } else { (x_next, x) };
            let x0_floor = x0.floor();
            let x0_index = x0_floor as i64;
            let x1_ceil = x1.ceil();
            let x1_index = x1_ceil as i64;

            if x1_index <= x0_index + 1 {
                let middle = 0.5 * (x + x_next) - x0_floor;

                add(row, x0_index, height - height * middle);
                add(row, x0_index + 1, height * middle);
            } else {
                let inverse = 1.0 / (x1 - x0);
                let x0_part = x0 - x0_floor;
                let first_cell_area = 0.5 * inverse * (1.0 - x0_part) * (1.0 - x0_part);
                let x1_part = x1 - x1_ceil + 1.0;
                let final_area = 0.5 * inverse * x1_part * x1_part;

                add(row, x0_index, height * first_cell_area);
                if x1_index == x0_index + 2 {
                    add(
                        row,
                        x0_index + 1,
                        height * (1.0 - first_cell_area - final_area),
                    );
                } else {
                    let cumulative_area = inverse * (1.5 - x0_part);

                    add(
                        row,
                        x0_index + 1,
                        height * (cumulative_area - first_cell_area),
                    );
                    for column in x0_index + 2..x1_index - 1 {
                        add(row, column, height * inverse);
                    }

                    let before_last = cumulative_area + (x1_index - x0_index - 3) as f64 * inverse;

                    add(row, x1_index - 1, height * (1.0 - before_last - final_area));
                }

                add(row, x1_index, height * final_area);
            }

            x = x_next;
        }
    }

    #[must_use]
    pub fn alpha(&self) -> &[u8] {
        &self.alpha
    }
}

#[cfg(test)]
#[path = "tests/coverage.rs"]
mod tests;
