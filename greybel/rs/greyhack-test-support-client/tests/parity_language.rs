//! The language and core functions: expressions, semantics and runtime errors.

use greybel_test_support::game::Game;

use crate::support::game::{setup_player, test_client};
use crate::support::parity::{game_outcomes, intrinsics_outcomes};

#[tokio::test]
async fn should_match_intrinsics_for_basic_game_scripts() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "print(\"hello \" + (1 + 1))",
        "print(\"one\"); print(\"two\")",
        "print(typeof(get_shell.host_computer))",
        "print(get_shell.host_computer.File(\"/bin/ls\").name)",
        "print(command_info(\"LS_USAGE\"))",
        "print(undefined_name)",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// Language behaviour the guide describes, where the game's MiniScript differs from the MiniScript manual or Greybel
/// once differed from the game.
/// The language's core intrinsics: math, string and list functions, `hash`, `shuffle`, `funcRef` and `yield`.
#[tokio::test]
async fn should_match_intrinsics_for_core_functions() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "print abs(-3.5); print abs(0); print abs(\"-2\"); print abs(null); print abs([1])",
        "print acos(0.5); print acos(2); print asin(1); print asin(-2); print atan(1); print atan(1, 2); print atan(-1, -1); print atan(0, 1); print atan(1, 0)",
        "print ceil(1.2); print ceil(-1.2); print floor(-1.5); print floor(2); print floor(\"a\"); print ceil(1e300 * 10)",
        "print cos(pi); print sin(pi / 2); print tan(pi / 4); print cos(\"x\"); print pi; print sqrt(2); print sqrt(-1); print sqrt(1e308 * 10)",
        "print round(2.5); print round(3.5); print round(-2.5); print round(0.125, 2); print round(1.23456, 3); print round(1234.5, -2); print round(1.55, 1.7); print round(2, -1); print round(\"x\")",
        "print sign(-3); print sign(0); print sign(2.5); print sign(\"x\"); print sign(-0); print sign(1 / 0)",
        "print sign(0 / 0)",
        "s = \"  Hello World  \"; print \"[\" + s.trim + \"]\"; print \"[\" + char(9) + \"a\" + char(10) + \"\".trim + \"]\"; print \"\".trim == \"\"",
        "print \"Ab1éß\".lower; print \"Ab1éß\".upper; print \"\".upper",
        "print \"xyz\".indexes; print [5, 6].indexes; print {\"a\": 1, \"b\": 2}.indexes; print [].indexes",
        "print val(\"12.5\"); print val(\"abc\"); print val(\"\"); print val(\"1e3\"); print val(\" 7 \"); print val(\"0x10\"); print val(\"1,5\"); print val(\"-0\"); print val(5); print val(null); print val(\"12abc\")",
        "print \"42\".to_int; print \"4.2\".to_int; print \"abc\".to_int; print \"-7\".to_int; print \" 7\".to_int; print \"99999999999\".to_int; print \"\".to_int; print \"+3\".to_int",
        "print \"abc\".is_match(\"b\"); print \"abc\".is_match(\"^b\"); print \"ABC\".is_match(\"b\", \"i\"); print \"a.c\".is_match(\"a\\.c\"); print \"x\".is_match(\"(\")",
        "print \"x\".is_match(\"\")",
        "print \"x\".is_match(null)",
        "print \"x\".is_match(\"x\", \"q\")",
        "print \"a1b22c\".replace(\"[0-9]+\", \"#\"); print \"aaa\".replace(\"a\", \"b\"); print \"ab\".replace(\"(a)(b)\", \"$2$1\"); print \"AbA\".replace(\"a\", \"x\", \"i\"); print \"a.b\".replace(\".\", \"-\")",
        "print \"x\".replace(\"(\", \"y\")",
        "print \"Hello\".replace(\"l\", \"L\", 1)",
        "print \"abc\".replace(\"\", \"x\")",
        "print \"abc\".replace(\"b\", null)",
        "print [1, 2, 1].replace(1, 3); print {\"a\": 1}.replace(1, 2)",
        "print slice(\"hello\", 1, 3); print slice(\"hello\", -2); print slice([1, 2, 3, 4], 1); print slice([1, 2, 3], 0, -1); print slice(\"abc\", 5); print slice(null, 1); print slice(5, 1)",
        "print [1, 2, 3].sum; print [1, \"2\", 3.5].sum; print [].sum; print {\"a\": 1, \"b\": 2}.sum; print [[1]].sum",
        "print hash(\"abc\"); print hash(\"abc\") == hash(\"abc\"); print hash(1); print hash(1.5); print hash([1, 2]); print hash({\"a\": 1}); print hash(null); print hash(\"\")",
        "x = [1, 2, 3, 4, 5]; x.shuffle; x.sort; print x; m = {\"a\": 1, \"b\": 2, \"c\": 3}; m.shuffle; print m.values.sum; print m.indexes.len; print [].shuffle",
        "f = funcRef; print typeof(f); print f.hasIndex(\"x\"); g = function; end function; print typeof(@g); print @g isa funcRef",
        "print 1; yield; print 2; for i in range(1, 3); yield; end for; print 3",
        "print typeof(number); print typeof(string); print typeof(list); print typeof(map); print \"a\" isa string; print 1 isa number; print [] isa list; print {} isa map",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);
    let actual = game_outcomes(&client, &scripts).await;
    let differences: Vec<_> = actual
        .iter()
        .zip(&expected)
        .filter(|(actual, expected)| actual != expected)
        .collect();

    assert!(differences.is_empty(), "{differences:#?}");
    client.shutdown();
}

#[tokio::test]
async fn should_match_intrinsics_for_language_semantics() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "print not 0.25; print not -2; print not 0; print not \"\"; print not \"a\"; print not [1]; print not null",
        "print 0.5 and 0.5; print 0.5 or 0.5; print 2 ^ 3 ^ 2",
        "print 0.1 + 0.2 == 0.3; print 0.1 + 0.2; print 1 / 3; print 1e20; print 1e-7; print 12345678901234567890",
        "x = 2; if (x > 1) then print \"parentheses\"",
        "print \"say \"\"hi\"\"\"; print \"a\\nb\"; print \"a\" + char(10) + \"b\"",
        "print \"abc\"[-1]; print \"hello\"[1:3]; print [1, 2, 3][5:]; print [1, 2, 3][-2:]",
        "a = [1, 2]; b = a.values; b.push(3); print a; c = a[0:]; c.push(4); print a",
        "print \"a.b.c\".split(\"\\.\"); print \"a.b.c\".replace(\"\\.\", \"-\")",
        "print \"a.b.c\".split(\".\")",
        "print \"abc\".lastIndexOf(\"z\"); print [1].indexOf(5); print [].pop; print [].pull; m = {\"a\": 1}; print m.remove(\"a\"); print m.remove(\"a\")",
        "print range(3); print range(1, 3)",
        "print range(1, 3, 0)",
        "f = function(c = -2, e = \"s\", n = null, t = true); return [c, e, n, t]; end function; print f",
        "C = {\"n\": 2}; C.m = function; return self.n; end function; c = new C; print c.m; f = @c.m; print f",
        "print str(@user_input); g = function(a, b = 1); end function; print str(@g)",
        "print typeof({\"classID\": \"foo\"}); print typeof(@print); print typeof(null)",
        "print bitAnd(3.9, 1.9); print bitOr(1, 2); print bitXor(3, 1)",
        "print bitAnd(2147483648, 2147483648); print bitAnd(2^33 + 5, 3); print bitOr(-2147483649, 0); print bitOr(0 / 0, 0); print bitOr(1e30, 0)",
        "C = {\"n\": 2}; C.m = function(a); return [self.n, self[\"n\"], a]; end function; c = new C; f = @c.m; print f(1); print [1, 2][1:].len",
        "f = function; return self; end function; print f",
        "f = function; return self.n; end function; print f",
        "x = null; print x.n",
        "for i in [5, 6]; print __i_idx; end for",
        "for x in [1, 2]; inside = x * 10; end for; print x; print inside",
        "r = [{\"n\": 10}, {\"n\": 2}]; r.sort(\"n\"); print r; p = [[10, \"b\"], [2, \"a\"]]; p.sort(0); print p",
        "m = {}; m[[1, 2]] = 1; print m.len",
        "print rnd(42); print rnd(42) == rnd(42)",
        "wait(0)",
        "print \"abc\".len; s = \"abc\"; s[0] = \"x\"",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// Runtime errors and the reads Grey Hack's MiniScript answers with null instead of an error.
#[tokio::test]
async fn should_match_intrinsics_for_runtime_errors() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "x = [1][5]",
        "print [1, 2][-3]",
        "x = \"abc\"[5]",
        "print [1][1.5]",
        "print [1][null]",
        "print \"abc\"[null]",
        "print [1][\"x\"]",
        "print [1][\"0\"]",
        "x = {\"a\": 1}.b",
        "x = {\"a\": 1}[\"b\"]",
        "print {}[1]",
        "x = {}; print x[null]",
        "print {}.foo.bar",
        "print \"abc\".foo",
        "x = 5; print x.len",
        "print (@print).foo",
        "print undefined_name",
        "x = undefined_func(1)",
        "x = null; print x.n",
        "x = null; print x.n.m",
        "f = function; return null; end function; print f.n",
        "x = null; print x[\"n\"]; print x[0]; print 5[0]",
        "print null.n; print (null).n; print @null.n; x = null.len; print x",
        "print null.n.m",
        "print 1 isa null; print \"a\" isa null; print {} isa null; x = number; print 1 isa null; print \"a\" isa null; print null isa null",
        "x = new 42; print typeof(x); print x.__isa; print new null; print new \"a\"",
        "f = function; return 5; end function; m = {\"f\": @f}; print (@m.f); print typeof((@m.f)); print (@f); m2 = {\"a\": m}; print (@m2.a.f)",
        "if 1 then if 0 then print 1 else print 2",
        "if 1 then if 1 then print 1 else print 2 else print 3",
        "print \"a\" * (1/0); print \"a\" * (0/0); print [\"a\"] * (0/0); print \"ab\" / 0; print \"ab\" * 2.5",
        "print [\"a\"] * (1/0)",
        "print [\"a\"] * 1e10",
        "x = \"ab\" * 9000000",
        "print null.n(1)",
        "f = function(a); end function; f(1, 2)",
        "x = 1; x(2)",
        "print(1, 2, 3)",
        "[1].push(1, 2)",
        "s = \"abc\"; s[0] = \"x\"",
        "x = null; x.a = 1",
        "x = null; x[0] = 1",
        "x = 5; x.a = 1",
        "x = \"abc\"; x.a = 1",
        "x = [1]; x[null] = 1",
        "x = [1]; x[5] = 2",
        "x = [1, 2]; x[-3] = 0",
        "x = [5]; x[\"a\"] = 1; print x",
        "x = [5, 6]; x[\"1\"] = 1; print x",
        "x = null; for i in x; print 1; end for; print 2",
        "print \"a\" - 1; print \"abc\" - \"c\"; print -\"a\"; print -[1]; print -null",
        "print \"a\" * \"b\"",
        "print \"abc\" / \"x\"",
        "print [1] + 1",
        "print [1] + null",
        "print {} + 1",
        "print [1] * \"x\"",
        "print [1] / \"x\"",
        "x = {}; x.__isa = x; print x.foo",
        "print get_shell.launch(\"/bin/ls\", null)",
        "print get_shell.launch(1)",
        "print get_shell.launch(\"\")",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}
