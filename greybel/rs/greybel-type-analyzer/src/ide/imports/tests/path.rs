#[path = "support/path.rs"]
mod support;

use greybel_core::Dialect;
use greybel_documents::{ImportKind, Uri};
use support::at;

use super::RelativePath;

#[test]
fn should_find_the_import_target_being_typed() {
    let include = "x = 1\n#include \"lib/a";
    let import = "#import Utils from \"";
    let native_import = "import_code(\"/lib";

    let include = at(include, Dialect::Greybel);
    let import = at(import, Dialect::Greybel);
    let native_import = at(native_import, Dialect::GreyScript);

    assert_eq!(include, Some((ImportKind::Include, "lib/a")));
    assert_eq!(import, Some((ImportKind::Import, "")));
    assert_eq!(native_import, Some((ImportKind::NativeImport, "/lib")));
}

#[test]
fn should_find_no_import_target_outside_an_open_import() {
    let native_import = "import_code(\"/lib";
    let closed = "#include \"a.src\" ";
    let in_a_string = "print \"#include ";
    let include = "#include \"a";

    let native_import_in_greybel = at(native_import, Dialect::Greybel);
    let closed = at(closed, Dialect::Greybel);
    let in_a_string = at(in_a_string, Dialect::Greybel);
    let include_in_miniscript = at(include, Dialect::MiniScript);

    assert_eq!(native_import_in_greybel, None);
    assert_eq!(closed, None);
    assert_eq!(in_a_string, None);
    assert_eq!(include_in_miniscript, None);
}

#[test]
fn should_relate_paths() {
    let between = |from: &str, to: &str| RelativePath::between_paths(from, to);

    let down = between("/main.src", "/lib/a.src");
    let sibling = between("/lib/a.src", "/lib/b.src");
    let up = between("/app/main.src", "/lib/a.src");
    let across = between("/a/b/c.src", "/a/d/e/f.src");

    assert_eq!(down, "lib/a.src");
    assert_eq!(sibling, "b.src");
    assert_eq!(up, "../lib/a.src");
    assert_eq!(across, "../d/e/f.src");
}

#[test]
fn should_relate_only_uris_of_one_root() {
    let main = Uri::memory("/app/main.src");

    let memory = RelativePath::between(&main, &Uri::memory("/lib/a.src"));
    let file = RelativePath::between(&main, &Uri::file("/lib/a.src"));

    assert_eq!(memory.as_deref(), Some("../lib/a.src"));
    assert_eq!(file, None);
}
