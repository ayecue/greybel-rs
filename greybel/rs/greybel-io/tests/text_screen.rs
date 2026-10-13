use greybel_io::{Output, PrintOptions, TextScreen};

use crate::support::text_screen::print;

#[test]
fn should_replace_a_full_line() {
    let mut screen = TextScreen::default();

    print(&mut screen, "10%", false);

    print(&mut screen, "20%", true);

    assert_eq!(screen.lines(), ["20%", ""]);
}

#[test]
fn should_replace_an_open_line_in_place() {
    let mut screen = TextScreen::default();

    screen.print(
        "loading",
        PrintOptions {
            new_line: false,
            replace: false,
        },
    );

    print(&mut screen, "done", true);

    assert_eq!(screen.lines(), ["done", ""]);
}

#[test]
fn should_only_print_when_replacing_on_a_blank_screen() {
    let mut screen = TextScreen::default();
    let options = PrintOptions {
        new_line: true,
        replace: false,
    };
    let batch = [
        Output::Print {
            text: "a".into(),
            options,
        },
        Output::Clear,
        Output::Print {
            text: "b".into(),
            options: PrintOptions {
                replace: true,
                ..options
            },
        },
    ];

    screen.apply(&batch);

    assert_eq!(screen.text(), "b\n");
}

#[test]
fn should_replace_only_the_last_of_several_lines() {
    let mut screen = TextScreen::default();

    print(&mut screen, "a\nb", false);

    print(&mut screen, "c", true);

    assert_eq!(screen.lines(), ["a", "c", ""]);
}
