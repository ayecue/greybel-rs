use crate::outline::{Outline, PointKind};
use crate::raster::{FlatPath, Subdivision};

/// Rectangles from `left`, `bottom` to `right`, `top`, each a counterclockwise contour.
pub fn rectangles(rectangles: &[(f64, f64, f64, f64)]) -> FlatPath {
    let fixed = |value: f64| (value * 64.0).round() as i64;
    let mut outline = Outline::default();

    for &(left, bottom, right, top) in rectangles {
        for (x, y) in [(left, bottom), (right, bottom), (right, top), (left, top)] {
            outline.push_point(fixed(x), fixed(y), PointKind::OnCurve);
        }

        outline.close_contour();
    }

    FlatPath::new(&outline, Subdivision::Precise)
}
