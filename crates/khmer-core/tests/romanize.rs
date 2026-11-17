//! Tests of romanized mode with the built-in data files.

use khmer_core::{is_valid, normalize, Romanizer};

const SENTENCES: &str = include_str!("data/sentences.tsv");

/// The roadmap's goal for the prototype: the right sentence is among the first
/// three suggestions for at least this share of the test sentences.
const TOP_3_GOAL: f32 = 0.80;

/// The test sentences as (typed, expected Khmer).
fn sentences() -> Vec<(&'static str, String)> {
    SENTENCES
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (typed, expected) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("sentences.tsv: no TAB in line: {line}"));
            (typed.trim(), normalize(expected.trim()))
        })
        .collect()
}

#[test]
fn built_in_data_loads_and_has_500_words() {
    let lexicon = Romanizer::builtin().lexicon();
    assert!(lexicon.len() >= 500, "only {} words", lexicon.len());
}

#[test]
fn design_doc_example() {
    let suggestions = Romanizer::builtin().convert("nh jg tv sala rean");
    assert_eq!(suggestions[0].text(), "ខ្ញុំចង់ទៅសាលារៀន");
    assert!(suggestions.len() <= 3);
}

#[test]
fn the_word_before_decides_between_words_spelled_the_same() {
    let top = |typed: &str| Romanizer::builtin().convert(typed)[0].text();
    // "nh" is ខ្ញុំ (I) at the start, and ញ៉ាំ (eat) after ចង់ (want).
    assert_eq!(top("nh jg tv"), "ខ្ញុំចង់ទៅ");
    assert_eq!(top("jg nh bay"), "ចង់ញ៉ាំបាយ");
    // "tv" is ទៅ (go), not ទីវី (TV), after ចង់.
    assert_eq!(top("jg tv"), "ចង់ទៅ");
    // "pros" is ប្រុស (male) after កូន (child), and ព្រោះ (because) before ខ្ញុំ.
    assert_eq!(top("kon pros"), "កូនប្រុស");
    assert_eq!(top("pros nh"), "ព្រោះខ្ញុំ");
}

#[test]
fn english_stays_english() {
    let top = |typed: &str| Romanizer::builtin().convert(typed)[0].text();
    assert_eq!(top("ok"), "ok");
    assert_eq!(top("facebook"), "facebook");
    assert_eq!(top("nh leng Facebook hz"), "ខ្ញុំលេង Facebook ហើយ");
}

#[test]
fn tokens_that_are_not_letters_are_kept() {
    let top = |typed: &str| Romanizer::builtin().convert(typed)[0].text();
    assert_eq!(top("mong 7"), "ម៉ោង 7");
    // Nothing typed, nothing suggested.
    assert!(Romanizer::builtin().convert("").is_empty());
    assert!(Romanizer::builtin().convert("   ").is_empty());
}

#[test]
fn every_suggestion_is_valid_normalized_khmer() {
    for (typed, _) in sentences() {
        for suggestion in Romanizer::builtin().convert(typed) {
            let text = suggestion.text();
            assert!(is_valid(&text), "{typed} -> {text}");
            assert_eq!(normalize(&text), text, "{typed}");
            assert_eq!(suggestion.words.len(), typed.split_whitespace().count());
        }
    }
}

#[test]
fn accuracy_on_the_test_sentences() {
    let sentences = sentences();
    assert!(sentences.len() >= 50, "only {} test sentences", sentences.len());

    let mut top_1 = 0;
    let mut top_3 = 0;
    let mut misses = Vec::new();
    for (typed, expected) in &sentences {
        let suggestions: Vec<String> =
            Romanizer::builtin().convert(typed).iter().map(|s| s.text()).collect();
        match suggestions.iter().position(|text| text == expected) {
            Some(0) => {
                top_1 += 1;
                top_3 += 1;
            }
            Some(_) => {
                top_3 += 1;
                misses.push(format!("  second or third: {typed} -> {}", suggestions[0]));
            }
            None => misses.push(format!(
                "  missing: {typed}\n    wanted {expected}\n    got    {}",
                suggestions.join(" / ")
            )),
        }
    }

    let total = sentences.len();
    let report = format!(
        "romanized accuracy on {total} sentences: first suggestion {top_1} ({:.0}%), \
         first three {top_3} ({:.0}%)\n{}",
        100.0 * top_1 as f32 / total as f32,
        100.0 * top_3 as f32 / total as f32,
        misses.join("\n")
    );
    // Shown with `cargo test -- --nocapture`.
    println!("{report}");
    assert!(top_3 as f32 / total as f32 >= TOP_3_GOAL, "{report}");
}
