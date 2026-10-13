use super::path::{FlatPath, Line};

/// Monochrome bitmap that sets pixels whose centers lie inside the outline by non-zero winding, stored as per-column row runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonochromeRaster {
    width: usize,
    height: usize,
    set_runs: Vec<Vec<(i64, i64)>>,
    clear_runs: Vec<Vec<(i64, i64)>>,
}

impl MonochromeRaster {
    /// Rasterizes `path` to a `width` by `height` bitmap with its left and top edges at `left` and `top` in y-up pixels.
    #[must_use]
    pub fn new(path: &FlatPath, left: f64, top: f64, width: usize, height: usize) -> Self {
        let mut lines: Vec<Line> = path
            .lines()
            .iter()
            .copied()
            .filter(|line| line.from.0 != line.to.0)
            .collect();

        lines.sort_by(|a, b| a.from.0.min(a.to.0).total_cmp(&b.from.0.min(b.to.0)));

        let mut set_runs = Vec::with_capacity(width);
        let mut active: Vec<Line> = Vec::new();
        let mut next = 0;
        let mut crossings: Vec<(f64, i32)> = Vec::new();

        for column in 0..width {
            let x = left + column as f64 + 0.5;

            while next < lines.len() && lines[next].from.0.min(lines[next].to.0) <= x {
                active.push(lines[next]);
                next += 1;
            }

            active.retain(|line| line.from.0.max(line.to.0) > x);
            crossings.clear();
            for line in &active {
                let ((x0, y0), (x1, y1)) = (line.from, line.to);

                if (x0 <= x && x < x1) || (x1 <= x && x < x0) {
                    let y = y0 + (x - x0) * (y1 - y0) / (x1 - x0);

                    crossings.push((y, if x1 > x0 { 1 } else { -1 }));
                }
            }

            crossings.sort_by(|a, b| b.0.total_cmp(&a.0));
            set_runs.push(Self::column_runs(&crossings, top, height));
        }

        let clear_runs = set_runs
            .iter()
            .map(|runs| Self::complement(runs, height))
            .collect();

        Self {
            width,
            height,
            set_runs,
            clear_runs,
        }
    }

    fn column_runs(crossings: &[(f64, i32)], top: f64, height: usize) -> Vec<(i64, i64)> {
        let mut runs: Vec<(i64, i64)> = Vec::new();
        let mut winding_number = 0;

        for pair in crossings.windows(2) {
            winding_number += pair[0].1;
            if winding_number == 0 {
                continue;
            }

            let (high, low) = (pair[0].0, pair[1].0);
            let start = ((top - 0.5 - high).floor() as i64 + 1).max(0);
            let end = ((top - 0.5 - low).floor() as i64 + 1).min(height as i64);

            if start >= end {
                continue;
            }

            match runs.last_mut() {
                Some(last) if last.1 >= start => last.1 = last.1.max(end),
                _ => runs.push((start, end)),
            }
        }

        runs
    }

    fn complement(runs: &[(i64, i64)], height: usize) -> Vec<(i64, i64)> {
        let mut clear_runs = Vec::with_capacity(runs.len() + 1);
        let mut row = 0;

        for &(start, end) in runs {
            if start > row {
                clear_runs.push((row, start));
            }

            row = end;
        }

        if row < height as i64 {
            clear_runs.push((row, height as i64));
        }

        clear_runs
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    #[must_use]
    pub fn is_set(&self, column: usize, row: usize) -> bool {
        let row = row as i64;

        self.set_runs
            .get(column)
            .is_some_and(|runs| runs.iter().any(|&(start, end)| start <= row && row < end))
    }

    /// Returns the smallest `|2 y + 1 - doubled_center|` across selected rows in `column` and the requested range: twice the nearest pixel-center distance.
    #[must_use]
    pub fn nearest_row(
        &self,
        column: usize,
        doubled_center: i64,
        first: i64,
        last: i64,
        is_set: bool,
    ) -> Option<i64> {
        let runs = if is_set {
            &self.set_runs
        } else {
            &self.clear_runs
        }
        .get(column)?;
        let below = (doubled_center - 1).div_euclid(2);
        let index = runs.partition_point(|&(_, end)| end <= below);
        let mut best: Option<i64> = None;

        for run in [index.checked_sub(1), Some(index)].into_iter().flatten() {
            let Some(&(start, end)) = runs.get(run) else {
                continue;
            };
            let (low, high) = (start.max(first), (end - 1).min(last));

            if low > high {
                continue;
            }

            for row in [below.clamp(low, high), (below + 1).clamp(low, high)] {
                let distance = (2 * row + 1 - doubled_center).abs();

                best = Some(best.map_or(distance, |best| best.min(distance)));
            }
        }

        best
    }
}

#[cfg(test)]
#[path = "tests/monochrome.rs"]
mod tests;
