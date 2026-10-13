/// Lower-envelope queries for the Felzenszwalb-Huttenlocher 1D distance transform.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LowerEnvelope {
    columns: Vec<usize>,
    values: Vec<i64>,
    starts: Vec<f64>,
}

impl LowerEnvelope {
    #[must_use]
    pub fn new(values: &[Option<i64>]) -> Self {
        let mut envelope = Self::default();

        for (column, value) in values.iter().enumerate() {
            if let Some(value) = *value {
                envelope.push(column, value);
            }
        }

        envelope
    }

    fn position(column: usize) -> i64 {
        2 * column as i64 + 1
    }

    fn push(&mut self, column: usize, value: i64) {
        let position = Self::position(column);

        while let (Some(&last), Some(&last_value), Some(&start)) =
            (self.columns.last(), self.values.last(), self.starts.last())
        {
            let last_position = Self::position(last);
            let intersection = ((value + position * position)
                - (last_value + last_position * last_position))
                as f64
                / (2 * (position - last_position)) as f64;

            if intersection <= start {
                self.columns.pop();
                self.values.pop();
                self.starts.pop();
                continue;
            }

            self.columns.push(column);
            self.values.push(value);
            self.starts.push(intersection);

            return;
        }

        self.columns.push(column);
        self.values.push(value);
        self.starts.push(f64::NEG_INFINITY);
    }

    /// Smallest value at `position` and the column it comes from.
    #[must_use]
    pub fn query(&self, position: i64) -> Option<(i64, usize)> {
        if self.columns.is_empty() {
            return None;
        }

        let index = self
            .starts
            .partition_point(|&start| start <= position as f64)
            .saturating_sub(1);
        let column = self.columns[index];
        let offset = position - Self::position(column);

        Some((offset * offset + self.values[index], column))
    }
}

#[cfg(test)]
#[path = "tests/envelope.rs"]
mod tests;
