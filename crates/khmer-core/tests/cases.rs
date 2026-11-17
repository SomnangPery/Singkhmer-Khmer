//! Runs every case in tests/data/cases.tsv. See the top of that file for the
//! format. To add a test, add a line there; no Rust needed.

use khmer_core::{is_valid, normalize, Engine};

const CASES: &str = include_str!("data/cases.tsv");

/// "179F 17D2" -> the string of those code points.
fn from_hex(hex: &str) -> String {
    hex.split_whitespace()
        .map(|h| {
            let code = u32::from_str_radix(h, 16).unwrap_or_else(|_| panic!("bad hex {h:?}"));
            char::from_u32(code).unwrap_or_else(|| panic!("not a character: {h}"))
        })
        .collect()
}

fn to_hex(text: &str) -> String {
    let codes: Vec<String> = text.chars().map(|c| format!("{:04X}", c as u32)).collect();
    codes.join(" ")
}

/// Types the keys in Khmer mode and returns everything committed to the app.
fn type_keys(keys: &str) -> String {
    let mut engine = Engine::new();
    let mut app = String::new();
    for key in keys.chars() {
        app.push_str(&engine.on_key(key).commit);
    }
    app.push_str(&engine.commit().commit);
    app
}

#[test]
fn all_cases_pass() {
    let mut failures = Vec::new();
    let mut count = 0;

    for (index, line) in CASES.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        assert!(
            columns.len() >= 3,
            "cases.tsv line {}: expected kind, input, expected (tab-separated)",
            index + 1
        );
        let (kind, input, expected) = (columns[0], from_hex(columns[1]), columns[2]);
        let note = columns.get(3).copied().unwrap_or("");
        count += 1;

        let (got, ok) = match kind {
            "normalize" => {
                let got = normalize(&input);
                // The result must be what we expect, and normalizing it again
                // must not change it.
                let ok = got == from_hex(expected) && normalize(&got) == got;
                (to_hex(&got), ok)
            }
            "valid" => {
                let got = if is_valid(&input) { "yes" } else { "no" };
                (got.to_string(), got == expected)
            }
            "type" => {
                let got = type_keys(&input);
                // Typed text must also always be valid and already normalized.
                let ok = got == from_hex(expected) && is_valid(&got) && normalize(&got) == got;
                (to_hex(&got), ok)
            }
            other => panic!("cases.tsv line {}: unknown kind {other:?}", index + 1),
        };

        if !ok {
            failures.push(format!(
                "line {} [{kind}] {}\n      got      {got}\n      expected {expected}\n      {note}",
                index + 1,
                columns[1]
            ));
        }
    }

    assert!(count > 0, "no cases found in cases.tsv");
    assert!(
        failures.is_empty(),
        "{} of {count} cases failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
