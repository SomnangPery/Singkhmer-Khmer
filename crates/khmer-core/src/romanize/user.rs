//! The user's own words: Singkhmer spellings they added themselves.
//!
//! One word per line, the spelling and the Khmer word separated by `=` or a
//! TAB, so the file can be read and edited in Notepad:
//!
//! ```text
//! # my words
//! bong = បង
//! oun = អូន
//! ```
//!
//! A typed token that is exactly one of these spellings (any capitals) gets
//! the user's word as the first suggestion, ahead of the built-in dictionary.
//! A spelling may be listed on several lines to give it several words; they
//! are suggested in the order of the file.

use std::collections::HashMap;
use std::sync::RwLock;

use super::lexicon::{data_lines, Candidate, DataError};
use crate::normalize::normalize;
use crate::validate::is_valid;

/// Above anything the built-in dictionary scores: a user word comes first.
/// Each further word for the same spelling scores a little lower, keeping the
/// order of the file.
const USER_WORD: f32 = 0.0;
const NEXT_USER_WORD: f32 = -0.01;

static USER_WORDS: RwLock<Option<HashMap<String, Vec<String>>>> = RwLock::new(None);

/// Replaces the user's words with those in `text` (the contents of their
/// words file). Lines that cannot be read are skipped and returned, so one
/// mistake does not lose the other words. Returns how many words were kept.
pub fn set_user_words(text: &str) -> (usize, Vec<DataError>) {
    let mut words: HashMap<String, Vec<String>> = HashMap::new();
    let mut skipped = Vec::new();
    let mut kept = 0;
    for (line, text) in data_lines(text) {
        match parse(text) {
            Ok((spelling, word)) => {
                let listed = words.entry(spelling).or_default();
                if !listed.contains(&word) {
                    listed.push(word);
                    kept += 1;
                }
            }
            Err(message) => skipped.push(DataError { file: "my words", line, message }),
        }
    }
    let mut store = USER_WORDS.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    *store = (!words.is_empty()).then_some(words);
    (kept, skipped)
}

/// Checks one line of the words file: `spelling = Khmer word`. For an editor
/// to check a word before saving it.
pub fn parse_user_word(line: &str) -> Result<(String, String), String> {
    parse(line)
}

fn parse(line: &str) -> Result<(String, String), String> {
    let (spelling, word) = line
        .split_once('\t')
        .or_else(|| line.split_once('='))
        .ok_or_else(|| "write the spelling, then = and the Khmer word".to_string())?;
    let spelling = spelling.trim().to_ascii_lowercase();
    let word = normalize(word.trim());
    if spelling.is_empty() || !spelling.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(format!("the spelling \"{spelling}\" should be letters a-z only, with no spaces"));
    }
    if word.is_empty() || !word.chars().all(|c| ('\u{1780}'..='\u{17FF}').contains(&c)) {
        return Err(format!("\"{word}\" should be Khmer letters only, with no spaces"));
    }
    if !is_valid(&word) {
        return Err(format!("\"{word}\" is not a correctly formed Khmer word"));
    }
    Ok((spelling, word))
}

/// The user's words for `token`, best first.
pub(crate) fn candidates(token: &str) -> Vec<Candidate> {
    let store = USER_WORDS.read().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(words) = store.as_ref() else {
        return Vec::new();
    };
    words
        .get(&token.to_ascii_lowercase())
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(rank, word)| Candidate {
            text: word.clone(),
            latin: false,
            score: USER_WORD + rank as f32 * NEXT_USER_WORD,
            fixed: true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_read_with_either_separator() {
        assert_eq!(parse("bong = បង").unwrap(), ("bong".to_string(), "បង".to_string()));
        assert_eq!(parse("Oun\tអូន").unwrap(), ("oun".to_string(), "អូន".to_string()));
    }

    #[test]
    fn bad_lines_say_what_is_wrong() {
        assert!(parse("bong").unwrap_err().contains("="));
        assert!(parse("bo ng = បង").unwrap_err().contains("a-z"));
        assert!(parse("bong = bong").unwrap_err().contains("Khmer"));
        assert!(parse("bong = \u{17B6}").unwrap_err().contains("correctly formed"));
    }
}
