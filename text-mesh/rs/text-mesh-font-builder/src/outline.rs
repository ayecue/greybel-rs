use crate::fixed_point::FixedPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointKind {
    OnCurve,
    Quadratic,
    Cubic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlinePoint {
    pub x: i64,
    pub y: i64,
    pub kind: PointKind,
}

/// Pixel extent of an outline's control box, including off-curve points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlBox {
    pub x_min: i64,
    pub y_min: i64,
    pub x_max: i64,
    pub y_max: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Line([(f64, f64); 2]),
    Quadratic([(f64, f64); 3]),
    Cubic([(f64, f64); 4]),
}

/// Glyph outline in 26.6 pixels with y up, laid out like FreeType's `FT_Outline`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outline {
    points: Vec<OutlinePoint>,
    contour_ends: Vec<usize>,
}

impl Outline {
    pub fn push_point(&mut self, x: i64, y: i64, kind: PointKind) {
        self.points.push(OutlinePoint { x, y, kind });
    }

    pub fn close_contour(&mut self) {
        let len = self.points.len();
        let start = self.contour_ends.last().map_or(0, |end| end + 1);

        if len > start {
            self.contour_ends.push(len - 1);
        }
    }

    pub fn append(&mut self, other: Self) {
        let offset = self.points.len();

        self.points.extend(other.points);
        self.contour_ends
            .extend(other.contour_ends.into_iter().map(|end| end + offset));
    }

    pub fn translate(&mut self, x: i64, y: i64) {
        for point in &mut self.points {
            point.x += x;
            point.y += y;
        }
    }

    pub fn transform(&mut self, xx: FixedPoint, xy: FixedPoint, yx: FixedPoint, yy: FixedPoint) {
        for point in &mut self.points {
            let (x, y) = (point.x, point.y);

            point.x = xx.scale(x) + xy.scale(y);
            point.y = yx.scale(x) + yy.scale(y);
        }
    }

    #[must_use]
    pub fn point(&self, index: usize) -> Option<OutlinePoint> {
        self.points.get(index).copied()
    }

    #[must_use]
    pub fn control_box(&self) -> Option<ControlBox> {
        let first = self.points.first()?;
        let start = ControlBox {
            x_min: first.x,
            y_min: first.y,
            x_max: first.x,
            y_max: first.y,
        };

        Some(self.points.iter().fold(start, |bounds, point| ControlBox {
            x_min: bounds.x_min.min(point.x),
            y_min: bounds.y_min.min(point.y),
            x_max: bounds.x_max.max(point.x),
            y_max: bounds.y_max.max(point.y),
        }))
    }

    /// Returns pixel curve segments for contours and inserts implied on-curve points between consecutive quadratic controls.
    #[must_use]
    pub fn segments(&self) -> Vec<Segment> {
        let mut segments = Vec::new();
        let mut start = 0;

        for &end in &self.contour_ends {
            self.contour_segments(&self.points[start..=end], &mut segments);
            start = end + 1;
        }

        segments
    }

    fn contour_segments(&self, contour: &[OutlinePoint], segments: &mut Vec<Segment>) {
        let pixel = |point: &OutlinePoint| (point.x as f64 / 64.0, point.y as f64 / 64.0);
        let Some(first_on) = contour
            .iter()
            .position(|point| point.kind == PointKind::OnCurve)
        else {
            self.quadratic_ring(contour, segments);

            return;
        };
        let count = contour.len();
        let origin = pixel(&contour[first_on]);
        let mut current = origin;
        let mut quadratic: Option<(f64, f64)> = None;
        let mut cubic: Vec<(f64, f64)> = Vec::new();

        for step in 1..=count {
            let point = &contour[(first_on + step) % count];
            let position = pixel(point);

            match point.kind {
                PointKind::OnCurve => {
                    segments.push(Self::segment(current, quadratic.take(), &cubic, position));
                    cubic.clear();
                    current = position;
                }
                PointKind::Quadratic => {
                    if let Some(control) = quadratic {
                        let middle = (
                            (control.0 + position.0) / 2.0,
                            (control.1 + position.1) / 2.0,
                        );

                        segments.push(Segment::Quadratic([current, control, middle]));
                        current = middle;
                    }

                    quadratic = Some(position);
                }
                PointKind::Cubic => cubic.push(position),
            }
        }
    }

    fn segment(
        from: (f64, f64),
        quadratic: Option<(f64, f64)>,
        cubic: &[(f64, f64)],
        to: (f64, f64),
    ) -> Segment {
        match (quadratic, cubic) {
            (Some(control), _) => Segment::Quadratic([from, control, to]),
            (None, [first, second, ..]) => Segment::Cubic([from, *first, *second, to]),
            (None, [control]) => Segment::Quadratic([from, *control, to]),
            (None, []) => Segment::Line([from, to]),
        }
    }

    fn quadratic_ring(&self, contour: &[OutlinePoint], segments: &mut Vec<Segment>) {
        let count = contour.len();
        let pixel = |index: usize| {
            let point = &contour[index % count];

            (point.x as f64 / 64.0, point.y as f64 / 64.0)
        };
        let middle = |a: (f64, f64), b: (f64, f64)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let origin = middle(pixel(count - 1), pixel(0));
        let mut current = origin;

        for index in 0..count {
            let control = pixel(index);
            let next = if index + 1 == count {
                origin
            } else {
                middle(control, pixel(index + 1))
            };

            segments.push(Segment::Quadratic([current, control, next]));
            current = next;
        }
    }
}

#[cfg(test)]
#[path = "tests/outline.rs"]
mod tests;
