use super::{Outline, PointKind, Segment};

#[test]
fn should_imply_on_curve_points_between_quadratic_controls() {
    let mut outline = Outline::default();

    outline.push_point(0, 0, PointKind::OnCurve);
    outline.push_point(64, 128, PointKind::Quadratic);
    outline.push_point(192, 128, PointKind::Quadratic);
    outline.push_point(256, 0, PointKind::OnCurve);
    outline.close_contour();

    let segments = outline.segments();

    assert_eq!(
        segments,
        vec![
            Segment::Quadratic([(0.0, 0.0), (1.0, 2.0), (2.0, 2.0)]),
            Segment::Quadratic([(2.0, 2.0), (3.0, 2.0), (4.0, 0.0)]),
            Segment::Line([(4.0, 0.0), (0.0, 0.0)]),
        ]
    );
}

#[test]
fn should_start_a_contour_without_on_curve_points_halfway_between_its_last_and_first() {
    let mut outline = Outline::default();

    for (x, y) in [(0, 0), (128, 0), (128, 128), (0, 128)] {
        outline.push_point(x, y, PointKind::Quadratic);
    }

    outline.close_contour();

    let segments = outline.segments();

    assert_eq!(segments.len(), 4);
    assert_eq!(
        segments[0],
        Segment::Quadratic([(0.0, 1.0), (0.0, 0.0), (1.0, 0.0)])
    );
}

#[test]
fn should_measure_the_control_box_over_off_curve_points() {
    let mut outline = Outline::default();

    outline.push_point(0, 0, PointKind::OnCurve);
    outline.push_point(32, 200, PointKind::Quadratic);
    outline.push_point(64, 0, PointKind::OnCurve);
    outline.close_contour();

    let bounds = outline.control_box();

    assert_eq!(bounds.map(|bounds| bounds.y_max), Some(200));
}
