//! Snapshots GreyScript fixtures and debugger cases; update with `INSTA_UPDATE=always cargo test`.

use greybel_core::Dialect;
use greybel_interpreter::RunResult;
use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::test_api::{run_as, run_with_eager_collection};
use greybel_test_support::interpreter::{resume_to_end, run_to_end};

use crate::support::greyscript::{MAIN, interpreter, interpreter_with_environment};
use crate::support::{pause, snapshot};

#[test]
fn should_pause_at_the_debugger_before_importing_code() {
    let (mut interpreter, _) = interpreter(&[
        (MAIN, "x = 1\ndebugger\nprint x + libValue"),
        ("lib", "libValue = 41"),
    ]);

    let result = pause(&mut interpreter);

    assert!(matches!(result, RunResult::Breakpoint(_)));
}

/// Code typed into the debugger console is GreyScript too; its `import_code` runs in the paused program's scope.
#[test]
fn should_import_code_from_injected_code() {
    let (mut interpreter, output) = interpreter(&[
        (MAIN, "x = 1\ndebugger\nprint x + libValue"),
        ("lib", "libValue = 41"),
    ]);

    pause(&mut interpreter);
    block_on(interpreter.inject("import_code(\"./lib\")\nprint libValue")).expect("injects");

    resume_to_end(&mut interpreter).expect("runs");

    assert_eq!(*output.borrow(), ["41", "42"]);
}

/// A default that isn't a literal is evaluated on each call that omits the argument or passes `null`, in the
/// function's scope.
#[test]
fn should_evaluate_expression_defaults_per_call() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "f = function(items = [], count = items.len + 1, callback = @g)\n  items.push(count)\n  return items + [callback]\nend function\ng = function\n  return \"g\"\nend function\nh = function\n  return \"h\"\nend function\nprint f\nprint f\nprint f([5], 0)\nprint f(null, null, @h)",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        *output.borrow(),
        ["[1, \"g\"]", "[1, \"g\"]", "[5, 0, \"g\"]", "[1, \"h\"]"]
    );
}

/// `a.f?.(x)` calls `f` on `a`, so `self` is `a`; `a[k]?.(x)` calls the function value; a `null` function ends the
/// chain with `null`.
#[test]
fn should_call_optional_methods_on_their_receiver() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "Counter = {\"n\": 1, \"cb\": null}\nCounter.add = function(x)\n  self.n = self.n + x\n  return self.n\nend function\nCounter.run = function\n  return [self.add?.(1), self.cb?.(1)]\nend function\nc = new Counter\ndouble = function(x)\n  return x * 2\nend function\nhandlers = {\"double\": @double}\nk = \"double\"\nprint [c.add?.(1), c.cb?.(1), c.n, handlers[k]?.(4), handlers[\"double\"]?.(5)]\nprint c.run\nnone = null\nprint [none?.add(1), none?.cb?.(1)]\nc.add?.(10)\nprint c.n",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        *output.borrow(),
        ["[2, null, 2, 8, 10]", "[3, null]", "[null, null]", "13"]
    );
}

/// Types written in code are skipped when running.
#[test]
fn should_run_code_with_types() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "let port: number = 22\nconnect = function(ip: string, port: number = 22): map|null\n  return {\"ip\": ip, \"port\": port}\nend function\nfirst = function<T>(items: list<T>): T\n  return items[0]\nend function\ndouble = (x: number) => x * 2\nvalue = [1, 2]!\nsize = value as list<number>\nprint [connect(\"1.2.3.4\").port, first([5, 6]), double(4), size, port]",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(*output.borrow(), ["[22, 5, 8, [1, 2], 22]"]);
}

/// A class is a map with its members; the constructor gives each instance its own copy of list fields and returns
/// `self`, and `create` makes an instance with it.
#[test]
fn should_run_classes() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "class Animal\n  tags = [0]\n  constructor(name)\n    self.name = name\n    if name == \"\" then return\n    self.named = 1\n  end function\n  speak()\n    return self.name + \": \" + self.sound\n  end function\n  static sound = \"...\"\nend class\nclass Dog extends Animal\n  static sound = \"woof\"\n  constructor(name)\n    super.constructor(name)\n  end function\nend class\nclass Point\n  items = [1]\nend class\na = Dog.create(\"rex\")\nb = Dog.create(\"max\")\na.tags[0] = 5\nprint [a.speak, b.speak, a.named, a.tags, b.tags, Dog.hasIndex(\"tags\"), Dog.sound]\ne = (new Animal).constructor(\"\")\nprint [e.name, e isa Animal, a isa Animal]\np = Point.create\nq = (new Point).constructor\np.items[0] = 2\nprint [p.items, q.items, Point.hasIndex(\"items\")]",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        *output.borrow(),
        [
            "[\"rex: woof\", \"max: woof\", 1, [5], [0], 0, \"woof\"]",
            "[\"\", 1, 1]",
            "[[2], [1], 0]"
        ]
    );
}

/// Greybel's syntax sugar runs as the plain MiniScript it stands for.
#[test]
fn should_run_greybel_syntax_sugar() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "let count = 0x10\nconst LIMIT = 1_000\nlet = 2\nprint count + LIMIT + let\nf = function\n  return \"f\"\nend function\nprint #\"{count}-{f} {{x}} \"\"q\"\" {#\"in{let}\"}\" == \"16-f {x} \"\"q\"\" in2\"\nprint #\"{null}\" == \"\"\nenum Color\n  Red\n  Green = 5\n  Blue\n  Name = \"n\"\nend enum\nprint [Color.Red, Color.Blue, Color.Name]\na = [1, 2]\nd = {\"x\": 1, \"y\": 2}\nname = \"n\"\nprint [...a, 3, ...a]\nprint [...a] == a\nm = {...d, \"y\": 5, name}\nprint [m.x, m.y, m.name, d.y, 0o17]\nassert m.x == 1, \"no\"\nassert = 3\nprint assert\ndouble = (x) => x * 2\nh = (a, b = 1) =>\n  return a + b\nend function\nprint [double(4), h(1), h(1, 2), [1, 2].len]\n[u, v] = \"a@b\".split(\"@\")\n[first, ...others] = [1, 2, 3]\n{x, y as why, z = 9, w = 4} = {\"x\": 1, \"y\": 2, \"w\": null}\nlet [k, [l, j]] = [1, [2, 3]]\nprint [u, v, first, others, x, why, z, w, k + l + j]\nout = []\nfor n, item in [\"a\", \"b\"]\n  out.push(n + item)\nend for\nfor [p, q] in [[1, 2], [3, 4]]\n  out.push(p * q)\nend for\nfor {key, value} in {\"k\": 5}\n  out.push(key + value)\nend for\nf = function\n  for n, [p] in [[7]]\n    return n + p\n  end for\nend function\nout.push(f)\nprint out\npick = function(v)\n  switch v\n    case 1, 2\n      return \"low\"\n    case is string\n      return \"text\"\n    else\n      return \"other\"\n  end switch\nend function\nprint [pick(2), pick(\"a\"), pick(9)]\nnone = null\ntripler = function(x)\n  return x * 3\nend function\nm = {\"n\": {\"k\": 5}, \"f\": @tripler}\ncb = function(x)\n  return x + 1\nend function\nnone?.run\nprint [1 ? \"t\" : \"f\", 0 ? \"t\" : 0 ? \"u\" : \"v\", none ?? \"d\", 0 ?? \"d\", m?.n.k, none?.n.k, m?.[\"n\"]?.k, none?.[0], cb?.(1), none?.(1), m.f?.(2), none?.f(2).x, \"abc\"?.[1:]]",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        *output.borrow(),
        [
            "1018",
            "1",
            "1",
            "[0, 6, \"n\"]",
            "[1, 2, 3, 1, 2]",
            "1",
            "[1, 5, \"n\", 2, 15]",
            "3",
            "[8, 2, 3, 2]",
            "[\"a\", \"b\", 1, [2, 3], 1, 2, 9, 4, 6]",
            "[\"0a\", \"1b\", 2, 12, \"k5\", 7]",
            "[\"low\", \"text\", \"other\"]",
            "[\"t\", \"v\", \"d\", 0, 5, null, 5, null, 2, null, 6, null, \"bc\"]",
        ]
    );
}

/// `#import {name, run as go} from "lib"` binds the exports to the names; `export` adds to the exports.
#[test]
fn should_bind_the_names_of_an_import_pattern() {
    let (mut interpreter, output) = interpreter(&[
        (
            MAIN,
            "#import {name, run as go} from \"lib\"\nprint [name, go]\n#import {twice} from \"exported\"\nprint twice(4)",
        ),
        (
            "lib",
            "run = function\n  return 2\nend function\nmodule.exports = {\"name\": \"lib\", \"run\": @run}",
        ),
        (
            "exported",
            "export twice = function(n)\n  return n * 2\nend function\nexport count = 1",
        ),
    ]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(*output.borrow(), ["[\"lib\", 2]", "8"]);
}

/// A failed assert calls `exit` with its message, or the file and line without one; `NO_ASSERTS` removes asserts.
#[test]
fn should_exit_at_a_failed_assert() {
    let exit = "exit = function(message)\n  print \"exit: \" + message\nend function\nx = 1\n";
    let run = |code: &str, environment: &[(&str, &str)]| {
        let source = format!("{exit}{code}");
        let (mut interpreter, printed) =
            interpreter_with_environment(&[(MAIN, source.as_str())], environment);

        run_to_end(&mut interpreter).expect("runs");

        printed.take()
    };

    let with_message = run("assert x > 1, \"too small\"\nassert x == 1", &[]);
    let with_location = run("assert x > 1", &[]);
    let removed = run("assert x > 1", &[("NO_ASSERTS", "1")]);

    assert_eq!(with_message, ["\"exit: too small\""]);
    assert_eq!(with_location, ["\"exit: Assertion failed at main:5\""]);
    assert_eq!(removed, Vec::<String>::new());
}

/// A map with a `classID` prints as it, wherever it appears; other maps print their entries.
#[test]
fn should_label_maps_by_their_class_id() {
    let (mut interpreter, output) = interpreter(&[(
        MAIN,
        "a = {\"classID\": \"Foo\"}\nprint [a, {\"inner\": a}]\nprint {\"plain\": 1}\nprint a",
    )]);

    run_to_end(&mut interpreter).expect("runs");
    assert_eq!(
        *output.borrow(),
        ["[Foo, {\"inner\": Foo}]", "{\"plain\": 1}", "Foo"]
    );
}

#[test]
fn should_run_every_script_fixture() {
    insta::glob!("fixtures/greyscript/scripts/*.src", |path| {
        let dialect = Dialect::GreyScript;
        let (printed, error) = run_as(path, dialect);
        let snapshot = snapshot(&printed, error);
        insta::assert_snapshot!(snapshot);
    });
}

/// Output is unchanged when the collector runs after every instruction; `fib.src` makes too many calls.
#[test]
fn should_print_the_same_when_collecting_after_every_instruction() {
    insta::glob!("fixtures/greyscript/scripts/*.src", |path| {
        if path.ends_with("fib.src") {
            return;
        }
        let expected = run_as(path, Dialect::GreyScript);
        let collected = run_with_eager_collection(path, Dialect::GreyScript);
        assert_eq!(collected, expected, "{}", path.display());
    });
}
