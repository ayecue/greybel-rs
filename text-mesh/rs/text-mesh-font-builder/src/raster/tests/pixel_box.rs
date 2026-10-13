use super::PixelBox;
use crate::outline::ControlBox;

#[test]
fn should_round_monochrome_edges_to_the_nearest_pixel() {
    let bounds = ControlBox {
        x_min: 64 + 31,
        y_min: -64 - 33,
        x_max: 5 * 64 + 31,
        y_max: 2 * 64 + 32,
    };

    let pixels = PixelBox::monochrome(Some(bounds));

    assert_eq!(
        pixels,
        PixelBox {
            left: 1,
            top: 3,
            width: 4,
            height: 5,
        }
    );
}

#[test]
fn should_grow_a_collapsed_monochrome_box_by_a_pixel() {
    let empty = None;

    let pixels = PixelBox::monochrome(empty);

    assert_eq!(
        pixels,
        PixelBox {
            left: 0,
            top: 1,
            width: 1,
            height: 1,
        }
    );
}

#[test]
fn should_move_smooth_edges_out_to_whole_pixels() {
    let bounds = ControlBox {
        x_min: 65,
        y_min: -1,
        x_max: 128,
        y_max: 129,
    };

    let pixels = PixelBox::smooth(Some(bounds));

    assert_eq!(
        pixels,
        PixelBox {
            left: 1,
            top: 3,
            width: 1,
            height: 4,
        }
    );
}
