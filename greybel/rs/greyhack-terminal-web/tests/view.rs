use greyhack_terminal_web::TerminalView;

use crate::support::{HEIGHT, WIDTH, output, view_of};

#[test]
fn should_follow_the_end_of_the_output() {
    let mut view = TerminalView::new(WIDTH, HEIGHT);

    view.set_text(&output(40));

    assert_eq!(
        (view.scroll(), view.is_following()),
        (view.content_height() - HEIGHT, true)
    );
}

#[test]
fn should_start_at_the_top_while_the_output_fits() {
    let mut view = TerminalView::new(WIDTH, HEIGHT);

    view.set_text(&output(2));

    assert_eq!((view.scroll(), view.is_following()), (0.0, true));
}

#[test]
fn should_stay_where_it_was_scrolled_to_while_the_output_grows() {
    let mut view = view_of(&output(40));

    view.scroll_by(-200.0);

    let scrolled = view.scroll();

    view.set_text(&output(60));

    assert_eq!((view.scroll(), view.is_following()), (scrolled, false));
}

#[test]
fn should_follow_again_once_scrolled_to_the_end() {
    let mut view = view_of(&output(40));

    view.scroll_by(-200.0);
    view.scroll_by(500.0);

    view.set_text(&output(60));

    assert_eq!(
        (view.scroll(), view.is_following()),
        (view.content_height() - HEIGHT, true)
    );
}

#[test]
fn should_not_scroll_above_the_output() {
    let mut view = view_of(&output(40));

    view.scroll_by(-1.0e6);

    assert_eq!((view.scroll(), view.is_following()), (0.0, false));
}

#[test]
fn should_split_the_output_into_the_games_terminal_lines() {
    let long = "x".repeat(300);

    let view = view_of(&format!("a\nb\n{long}"));

    assert_eq!(view.line_count(), 4);
}

#[test]
fn should_draw_grown_output_as_if_laid_out_at_once() {
    let mut view = view_of(&output(30));

    view.set_text(&output(50));

    assert_eq!(view.meshes(), view_of(&output(50)).meshes());
}

#[test]
fn should_move_the_lines_after_one_changing_its_height() {
    let mut view = view_of(&output(30));
    let taller = output(30).replacen("line</color> 5", "<size=40>tall</size>", 1);

    view.set_text(&taller);

    assert_eq!(
        (view.content_height(), view.meshes()),
        (view_of(&taller).content_height(), view_of(&taller).meshes())
    );
}

#[test]
fn should_lay_the_lines_out_again_at_a_new_width() {
    let text = format!("{} {}", "word".repeat(60), output(10));
    let mut view = view_of(&text);

    view.resize(400.0, HEIGHT);

    let mut fresh = TerminalView::new(400.0, HEIGHT);

    fresh.set_text(&text);
    assert_eq!(
        (view.content_height(), view.meshes()),
        (fresh.content_height(), fresh.meshes())
    );
}

#[test]
fn should_draw_only_the_lines_in_and_near_sight() {
    let near = view_of(&output(20));

    let far = view_of(&output(400));

    let quads = |view: &TerminalView| {
        view.meshes()
            .iter()
            .map(text_mesh_webgl::Mesh::vertex_count)
            .sum::<usize>()
    };

    assert!(
        quads(&far) <= 2 * quads(&near),
        "{} vs {}",
        quads(&far),
        quads(&near)
    );
}
