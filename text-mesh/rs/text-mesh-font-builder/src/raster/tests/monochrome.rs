use super::MonochromeRaster;
use crate::test_support::rectangles;

#[test]
fn should_set_the_pixels_whose_centers_are_inside() {
    let path = rectangles(&[(1.2, 0.4, 3.7, 2.6)]);

    let raster = MonochromeRaster::new(&path, 0.0, 4.0, 5, 4);

    let set: Vec<(usize, usize)> = (0..4)
        .flat_map(|row| (0..5).map(move |column| (column, row)))
        .filter(|&(column, row)| raster.is_set(column, row))
        .collect();

    assert_eq!(
        set,
        vec![
            (1, 1),
            (2, 1),
            (3, 1),
            (1, 2),
            (2, 2),
            (3, 2),
            (1, 3),
            (2, 3),
            (3, 3)
        ]
    );
}

#[test]
fn should_fill_overlapping_contours_once() {
    let path = rectangles(&[(0.0, 0.0, 2.0, 2.0), (1.0, 1.0, 3.0, 3.0)]);

    let raster = MonochromeRaster::new(&path, 0.0, 3.0, 3, 3);

    let set: Vec<bool> = (0..3)
        .flat_map(|row| (0..3).map(move |column| (column, row)))
        .map(|(column, row)| raster.is_set(column, row))
        .collect();

    assert_eq!(
        set,
        vec![false, true, true, true, true, true, true, true, false]
    );
}

#[test]
fn should_find_the_nearest_row_of_a_state_inside_the_window() {
    let path = rectangles(&[(0.0, 0.0, 1.0, 2.0)]);
    let raster = MonochromeRaster::new(&path, 0.0, 10.0, 1, 10);

    let nearest = [
        raster.nearest_row(0, 2 * 3 + 1, 0, 9, true),
        raster.nearest_row(0, 2 * 3 + 1, 0, 7, true),
        raster.nearest_row(0, 2 * 9, 0, 9, true),
        raster.nearest_row(0, 2 * 9, 0, 9, false),
    ];

    assert_eq!(nearest, [Some(10), None, Some(1), Some(3)]);
}
