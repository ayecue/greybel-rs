//! Derives a scenario's `requires` entries from its options, steps, and frames.

use std::collections::BTreeSet;

use serde_json::Value;

/// Requirement names in schema order.
pub const REQUIREMENTS: [&str; 10] = [
    "responses",
    "streams",
    "cancellation",
    "timeouts",
    "migration",
    "handshake",
    "security",
    "limits",
    "health-check",
    "fragments",
];

const RESPONSE_FRAMES: [&str; 4] = [
    "ResponseEnd",
    "ResponseStart",
    "ResponseItem",
    "ResponseCredit",
];
const STREAM_FRAMES: [&str; 6] = [
    "MessageItem",
    "MessageEnd",
    "MessageCredit",
    "ResponseStart",
    "ResponseItem",
    "ResponseCredit",
];
/// `Failure` fields that indicate rejection rather than a declared failure.
const REJECTION_FIELDS: [&str; 3] = ["id", "rejected", "violation"];

/// Returns the requirements used by `scenario` in schema order.
pub fn required(scenario: &Value) -> Vec<&'static str> {
    let mut found = BTreeSet::new();
    let steps = scenario["steps"].as_array().map_or(&[][..], Vec::as_slice);
    let mut sent = Vec::new();
    let mut received = Vec::new();

    for step in steps {
        if let Some(batch) = step.get("receive") {
            received.extend(frames(batch));
        }

        if let Some(expectation) = step.get("expect") {
            for batch in expectation["sent"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
            {
                sent.extend(frames(batch));
            }

            if expectation.get("settled").is_some() {
                found.insert("responses");
            }
        }

        for (key, requirement) in [
            ("take", "streams"),
            ("item", "streams"),
            ("end", "streams"),
            ("drop", "cancellation"),
            ("wait", "timeouts"),
            ("check", "health-check"),
        ] {
            if step.get(key).is_some() {
                found.insert(requirement);
            }
        }
    }

    for (kind, _) in sent.iter().chain(&received) {
        if STREAM_FRAMES.contains(kind) {
            found.insert("streams");
        }

        if RESPONSE_FRAMES.contains(kind) {
            found.insert("responses");
        }

        match *kind {
            "Cancel" => {
                found.insert("cancellation");
            }
            "Fragment" => {
                found.insert("fragments");
            }
            _ => {}
        }
    }

    for (kind, frame) in &sent {
        let declared_failure = *kind == "Failure"
            && frame.as_object().is_some_and(|fields| {
                fields
                    .keys()
                    .any(|key| !REJECTION_FIELDS.contains(&key.as_str()))
            });
        let awaits_response = *kind == "Message" && frame.get("id").is_some();

        if declared_failure || awaits_response {
            found.insert("responses");
        }
    }

    for (option, requirement) in [
        ("streamWindow", "streams"),
        ("security", "security"),
        ("limits", "limits"),
    ] {
        if scenario.get(option).is_some() {
            found.insert(requirement);
        }
    }

    if scenario["major"] == 2 {
        found.insert("migration");
    }

    if scenario["protocol"] == "login" {
        found.insert("handshake");
    }

    if scenario["healthCheck"] == true {
        found.insert("health-check");
    }

    REQUIREMENTS
        .into_iter()
        .filter(|requirement| found.contains(requirement))
        .collect()
}

/// Returns `(kind, frame)` pairs for `batch`, including each fragment's wrapped frame.
fn frames(batch: &Value) -> Vec<(&str, &Value)> {
    let mut found = Vec::new();

    for frame in batch["frames"].as_array().map_or(&[][..], Vec::as_slice) {
        for (kind, value) in frame.as_object().into_iter().flatten() {
            found.push((kind.as_str(), value));
            if kind == "Fragment" {
                for (inner, value) in value["of"].as_object().into_iter().flatten() {
                    found.push((inner.as_str(), value));
                }
            }
        }
    }

    found
}
