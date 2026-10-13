use crate::support::scenarios::requirements::required;
use crate::support::scenarios::runner::Scenario;
use crate::support::scenarios::scenario_files;

#[test]
fn should_pass_every_shared_scenario() {
    let scenarios = scenario_files();

    let failures: Vec<String> = scenarios
        .iter()
        .filter_map(|(name, scenario)| {
            Scenario::run(scenario)
                .err()
                .map(|error| format!("{name}: {error}"))
        })
        .collect();

    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn should_require_what_every_scenario_uses() {
    let scenarios = scenario_files();

    let mismatches: Vec<String> = scenarios
        .iter()
        .filter_map(|(name, scenario)| {
            let declared: Vec<&str> = scenario["requires"]
                .as_array()
                .map(|requirements| {
                    requirements
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect()
                })
                .unwrap_or_default();
            let used = required(scenario);

            (declared != used).then(|| format!("{name}: requires {declared:?}, uses {used:?}"))
        })
        .collect();

    assert!(mismatches.is_empty(), "\n{}", mismatches.join("\n"));
}
