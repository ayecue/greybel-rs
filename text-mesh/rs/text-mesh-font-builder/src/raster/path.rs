use crate::outline::{Outline, Segment};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub from: (f64, f64),
    pub to: (f64, f64),
}

/// Flattens curves using exact or FreeType-compatible subdivision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subdivision {
    Precise,
    FreeTypeGray,
}

/// Outline as closed polylines in pixels, y up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlatPath {
    lines: Vec<Line>,
}

impl Subdivision {
    const GRAY_LIMIT: i64 = 64;
    const GRAY_SUBPIXELS: f64 = 256.0;
    const TOLERANCE: f64 = 0.02;

    fn quadratic_steps(self, from: (f64, f64), control: (f64, f64), to: (f64, f64)) -> u32 {
        let second_difference = (
            from.0 - 2.0 * control.0 + to.0,
            from.1 - 2.0 * control.1 + to.1,
        );

        match self {
            Self::Precise => {
                Self::precise_steps(second_difference.0.hypot(second_difference.1) / 4.0)
            }
            Self::FreeTypeGray => {
                let mut deviation = (second_difference.0.abs().max(second_difference.1.abs())
                    * Self::GRAY_SUBPIXELS) as i64;
                let mut steps = 1;

                while deviation > Self::GRAY_LIMIT {
                    deviation >>= 2;
                    steps <<= 1;
                }

                steps
            }
        }
    }

    fn cubic_steps(points: [(f64, f64); 4]) -> u32 {
        let second_difference = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (a.0 - 2.0 * b.0 + c.0).hypot(a.1 - 2.0 * b.1 + c.1)
        };
        let deviation = second_difference(points[0], points[1], points[2])
            .max(second_difference(points[1], points[2], points[3]))
            * 0.75;

        Self::precise_steps(deviation)
    }

    fn precise_steps(deviation: f64) -> u32 {
        ((deviation / Self::TOLERANCE).sqrt().ceil() as u32).max(1)
    }
}

impl FlatPath {
    #[must_use]
    pub fn new(outline: &Outline, subdivision: Subdivision) -> Self {
        let mut lines = Vec::new();

        for segment in outline.segments() {
            match segment {
                Segment::Line([from, to]) => lines.push(Line { from, to }),
                Segment::Quadratic([from, control, to]) => {
                    let steps = subdivision.quadratic_steps(from, control, to);

                    Self::subdivide(&mut lines, from, steps, |t| {
                        let u = 1.0 - t;

                        (
                            u * u * from.0 + 2.0 * u * t * control.0 + t * t * to.0,
                            u * u * from.1 + 2.0 * u * t * control.1 + t * t * to.1,
                        )
                    });
                }
                Segment::Cubic(points) => {
                    let steps = Subdivision::cubic_steps(points);
                    let [a, b, c, d] = points;

                    Self::subdivide(&mut lines, a, steps, |t| {
                        let u = 1.0 - t;
                        let (w0, w1, w2, w3) =
                            (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);

                        (
                            w0 * a.0 + w1 * b.0 + w2 * c.0 + w3 * d.0,
                            w0 * a.1 + w1 * b.1 + w2 * c.1 + w3 * d.1,
                        )
                    });
                }
            }
        }

        Self { lines }
    }

    fn subdivide(
        lines: &mut Vec<Line>,
        start: (f64, f64),
        steps: u32,
        point: impl Fn(f64) -> (f64, f64),
    ) {
        let mut from = start;

        for step in 1..=steps {
            let to = point(f64::from(step) / f64::from(steps));

            lines.push(Line { from, to });
            from = to;
        }
    }

    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
}
