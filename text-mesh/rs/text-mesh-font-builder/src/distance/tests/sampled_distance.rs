use super::SampledDistance;
use crate::raster::MonochromeRaster;
use crate::test_support::rectangles;

#[test]
fn should_measure_from_block_centers_within_the_window() {
    let path = rectangles(&[(0.0, -10.0, 6.0, 10.0)]);
    let raster = MonochromeRaster::new(&path, 0.0, 2.0, 12, 2);

    let alpha = SampledDistance::new(&raster, 2, 2).alpha(6, 1);

    assert_eq!(alpha, vec![255, 240, 178, 77, 15, 0]);
}
