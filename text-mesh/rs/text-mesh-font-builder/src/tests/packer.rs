use super::{AtlasPacker, PackedRect};

#[test]
fn should_place_the_first_rectangle_at_the_origin() {
    let mut packer = AtlasPacker::new(100, 100);

    let placed = packer.insert(10, 20);

    assert_eq!(
        placed,
        Some(PackedRect {
            x: 0,
            y: 0,
            width: 10,
            height: 20,
        })
    );
}

#[test]
fn should_place_like_unitys_best_short_side_fit() {
    let mut packer = AtlasPacker::new(1023, 1023);
    let sizes = [(20, 20), (28, 79), (43, 38), (67, 78)];

    let positions: Vec<Option<(u32, u32)>> = sizes
        .iter()
        .map(|&(width, height)| {
            packer
                .insert(width, height)
                .map(|placed| (placed.x, placed.y))
        })
        .collect();

    assert_eq!(
        positions,
        vec![Some((0, 0)), Some((0, 20)), Some((0, 99)), Some((28, 0))]
    );
}

#[test]
fn should_refuse_a_rectangle_no_free_space_contains() {
    let mut packer = AtlasPacker::new(50, 50);

    packer.insert(50, 30);

    let placed = packer.insert(30, 30);

    assert_eq!(placed, None);
}

#[test]
fn should_never_overlap_placed_rectangles() {
    let mut packer = AtlasPacker::new(256, 256);
    let sizes: Vec<(u32, u32)> = (0..200u32)
        .map(|index| (5 + (index * 37) % 23, 5 + (index * 53) % 29))
        .collect();

    let placed: Vec<PackedRect> = sizes
        .iter()
        .filter_map(|&(width, height)| packer.insert(width, height))
        .collect();

    assert!(placed.len() > 50);
    for (index, first) in placed.iter().enumerate() {
        assert!(first.right() <= 256 && first.top() <= 256, "{first:?}");
        for second in &placed[index + 1..] {
            assert!(!first.intersects(*second), "{first:?} {second:?}");
        }
    }
}
