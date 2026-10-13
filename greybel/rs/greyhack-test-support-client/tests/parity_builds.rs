//! What the game compiles: syntax it accepts, compiler errors and the imports of a build.

use greybel_test_support::game::Game;

use crate::support::game::{setup_player, test_client};
use crate::support::parity::{Outcome, game_outcomes, intrinsics_outcomes};

/// How `build` puts the files `import_code` names in place (`PlayerUtils.ImportCode`): a call right after `//` stays a
/// comment, one later in a comment is still imported, and an imported file's own `import_code` stays in the built code,
/// where it is an unknown name. The failure's own lines are left out: their text differs.
#[tokio::test]
async fn should_match_intrinsics_for_the_imports_of_a_build() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let script = [
        "sh = get_shell; pc = sh.host_computer",
        "write = function(name, lines); pc.touch(home_dir, name); pc.File(home_dir + \"/\" + name).set_content(lines.join(char(10))); end function",
        "call = function(name); return \"import_code(\"\"\" + home_dir + \"/\" + name + \"\"\")\"; end function",
        "write(\"lib.src\", [\"x = 0\", \"lib_loaded = 1\"])",
        "write(\"deep.src\", [\"deep = 1\"])",
        "write(\"nested.src\", [call(\"deep.src\")])",
        "write(\"spaced.src\", [\"// \" + call(\"lib.src\"), \"print lib_loaded\"])",
        "write(\"skipped.src\", [\"//\" + call(\"lib.src\"), \"print 2\"])",
        "write(\"outer.src\", [call(\"nested.src\"), \"get_custom_object.ran = 1\"])",
        "for name in [\"spaced\", \"skipped\", \"outer\"]",
        "  print name + \" build=\" + sh.build(home_dir + \"/\" + name + \".src\", home_dir)",
        "  print name + \" launch=\" + sh.launch(home_dir + \"/\" + name)",
        "end for",
        "print \"ran=\" + get_custom_object.hasIndex(\"ran\")",
    ]
    .join("\n");
    let printed = |outcome: &Outcome| {
        outcome.output.as_deref().map(|output| {
            output
                .lines()
                .filter(|line| !line.starts_with("<color=red>"))
                .collect::<Vec<_>>()
                .join("\n")
        })
    };
    let expected = intrinsics_outcomes(&[script.as_str()], &user);

    let actual = game_outcomes(&client, &[script.as_str()]).await;

    assert_eq!(printed(&actual[0]), printed(&expected[0]));
    assert_eq!(
        printed(&expected[0]).as_deref(),
        Some(
            "spaced build=\n1\nspaced launch=1\nskipped build=\n2\nskipped launch=1\nouter build=\nouter launch=1\nran=0"
        )
    );
    client.shutdown();
}

/// What the game's own compiler accepts: Greybel's additions (bitwise operators, compound assignment, block comments,
/// `#envar`, `#line`, `#filename`) fail to build, and `debugger` is an ordinary name, as for code a player writes.
#[tokio::test]
async fn should_match_intrinsics_for_code_the_game_compiles() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let sources = [
        "x = 5 & 3",
        "x = 5 | 3",
        "x = 1 << 2",
        "x = 1\nx += 1",
        "/* block */\nx = 1",
        "x = #envar HOME",
        "x = #line",
        "x = #filename",
        "f(1,\n)",
        "debugger = 2\nprint debugger",
    ];
    let script = [
        "sh = get_shell; pc = sh.host_computer".to_owned(),
        format!(
            "sources = [{}]",
            sources
                .iter()
                .map(|source| format!("\"{}\"", source.replace('"', "\"\"").replace('\n', "\" + char(10) + \"")))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "for i in range(0, sources.len - 1)".to_owned(),
        "  pc.touch(home_dir, \"c\" + i + \".src\"); pc.File(home_dir + \"/c\" + i + \".src\").set_content(sources[i])".to_owned(),
        "  built = sh.build(home_dir + \"/c\" + i + \".src\", home_dir)".to_owned(),
        "  print i + \" \" + (built == \"\")".to_owned(),
        "  if built == \"\" then sh.launch(home_dir + \"/c\" + i)".to_owned(),
        "end for".to_owned(),
    ]
    .join("\n");
    let expected = intrinsics_outcomes(&[script.as_str()], &user);

    let actual = game_outcomes(&client, &[script.as_str()]).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

#[tokio::test]
async fn should_match_intrinsics_for_the_errors_of_a_build() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let sources = [
        "x = (1",
        "if true then\nprint 1",
        "end if",
        "x = 1 +",
        "x = [1, 2",
        "x = {1:}",
        "x == = 2",
        "break",
        "else",
        "for i in\nend for",
        "x = 1 2",
        "print \"abc",
        "f = function(a\nend function",
        "while true\nend for",
        "x = @",
        "x = 1\n\ny = (1\n",
        "x = [1,\n+]",
        "print 1\nend function\n",
        "x = 1\nwhile true\nend for",
        "if true then\nx = 1\nend if\nwhile true\n",
        "continue",
        "x = 1\nprint \"abc",
        "x = 1\nif x then\n  y = (2\nend if",
        "import_code(\"/missing.src\")",
    ];
    let script = [
        "sh = get_shell; pc = sh.host_computer".to_owned(),
        format!(
            "sources = [{}]",
            sources
                .iter()
                .map(|source| format!("\"{}\"", source.replace('"', "\"\"").replace('\n', "\" + char(10) + \"")))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        "for i in range(0, sources.len - 1)".to_owned(),
        "  pc.touch(home_dir, \"e\" + i + \".src\"); pc.File(home_dir + \"/e\" + i + \".src\").set_content(sources[i])".to_owned(),
        "  print i + \" \" + sh.build(home_dir + \"/e\" + i + \".src\", home_dir, 1)".to_owned(),
        "end for".to_owned(),
    ]
    .join("\n");
    let expected = intrinsics_outcomes(&[script.as_str()], &user);

    let actual = game_outcomes(&client, &[script.as_str()]).await;

    assert_eq!(actual, expected);
    client.shutdown();
}
