//! Replays the changes in `fixtures/dictionary.json`, comparing the order after each with Grey Hack's runtime.

use serde_json::Value as Json;
use support::replay;

use crate::support;

#[test]
fn should_enumerate_like_the_game() {
    let fixtures: Json =
        serde_json::from_str(include_str!("fixtures/dictionary.json")).expect("fixtures");
    let series = fixtures["series"].as_array().expect("series");
    let steps = replay(series);

    for recorded in &steps {
        let (number_of_series, step, change) =
            (recorded.number_of_series, recorded.step, &recorded.change);

        assert!(
            recorded.applied,
            "series {number_of_series} step {step}: {change:?}"
        );
        assert_eq!(
            recorded.actual, recorded.expected,
            "series {number_of_series} step {step}: {change:?}"
        );
        assert_eq!(recorded.len, recorded.expected.len());
        for (position, entry) in recorded.expected.iter().enumerate() {
            assert_eq!(recorded.indexed[position], Some(*entry));
        }

        assert_eq!(recorded.past_the_end, None);
    }

    assert_eq!(steps.len(), 16 * 150);
}
