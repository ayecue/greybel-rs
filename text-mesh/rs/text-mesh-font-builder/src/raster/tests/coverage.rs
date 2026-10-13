use super::CoverageRaster;
use crate::test_support::rectangles;

#[test]
fn should_store_the_covered_area_of_each_pixel() {
    let path = rectangles(&[(0.5, 0.0, 2.5, 1.0), (3.0, 0.0, 3.25, 0.5)]);

    let raster = CoverageRaster::new(&path, 0.0, 1.0, 4, 1);

    assert_eq!(raster.alpha(), &[128, 255, 128, 32]);
}
