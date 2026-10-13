//! Snapshots every Greybel script in `tests/fixtures/greybel/scripts/`; update with `INSTA_UPDATE=always cargo test`.

use greybel_core::Dialect;
use greybel_test_support::interpreter::test_api::{run, run_as, run_with_eager_collection};

use crate::support::snapshot;

#[test]
fn should_run_every_script_fixture() {
    insta::glob!("fixtures/greybel/scripts/*.src", |path| {
        let dialect = Dialect::Greybel;
        let (printed, error) = run_as(path, dialect);
        let snapshot = snapshot(&printed, error);
        insta::assert_snapshot!(snapshot);
    });
}

/// Output is unchanged when the collector runs after every instruction; `fib.src` makes too many calls.
#[test]
fn should_print_the_same_when_collecting_after_every_instruction() {
    insta::glob!("fixtures/greybel/scripts/*.src", |path| {
        if path.ends_with("fib.src") {
            return;
        }
        let expected = run(path);
        let collected = run_with_eager_collection(path, Dialect::Greybel);
        assert_eq!(collected, expected, "{}", path.display());
    });
}
