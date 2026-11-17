//! The language model: how likely a word is after the words before it.
//!
//! [`LanguageModel`] is the interface the converter uses. [`SeedModel`] is a
//! small stand-in built from hand-written estimates. The real model, counted
//! from a text corpus (roadmap step 2), will implement the same trait, so the
//! rest of the engine does not change.

use std::collections::HashMap;

use super::lexicon::{data_lines, DataError, Lexicon};
use crate::normalize::normalize;

/// Scores a word given the words before it.
pub trait LanguageModel: Send + Sync {
    /// The natural logarithm of the probability of `word` coming next.
    ///
    /// `previous` holds up to two words that come before it, the nearest one
    /// last. It is empty at the start of a sentence. A word the model does not
    /// know (for example one kept in Latin letters) may appear in `previous`.
    fn log_prob(&self, previous: &[&str], word: &str) -> f32;
}

/// How the start of a sentence is written in `pairs.tsv`.
const START: &str = "<s>";
/// Weight of a word's overall frequency when its pair is not listed.
const BACKOFF: f32 = 0.4;
/// Probability given to a word the model has never seen.
const UNSEEN: f32 = 1e-7;

/// A stand-in language model made from two hand-written tables:
///
/// * how common each word is (the count column of `words.tsv`),
/// * which word pairs go together (`data/pairs.tsv`).
///
/// `pairs.tsv` has one pair per line:
///
/// ```text
/// first word <TAB> next word <TAB> percent
/// ចង់         <TAB> ទៅ        <TAB> 12
/// ```
///
/// meaning "about 12% of the time, the word after ចង់ is ទៅ". `<s>` as the
/// first word means the start of a sentence. It only looks one word back.
pub struct SeedModel {
    /// Word -> probability of the word on its own.
    alone: HashMap<String, f32>,
    /// First word -> next word -> probability of that next word.
    pairs: HashMap<String, HashMap<String, f32>>,
}

impl SeedModel {
    /// Builds the model from the dictionary's word counts and the text of
    /// `pairs.tsv`. Every word in a pair must be in the dictionary.
    pub fn new(lexicon: &Lexicon, pairs: &str) -> Result<SeedModel, DataError> {
        let total: f32 = lexicon.words().map(|(_, count)| count).sum();
        let alone = lexicon
            .words()
            .map(|(word, count)| (word.to_string(), count / total))
            .collect();
        let mut model = SeedModel { alone, pairs: HashMap::new() };
        for (line, text) in data_lines(pairs) {
            model
                .add_pair(lexicon, text)
                .map_err(|message| DataError { file: "pairs", line, message })?;
        }
        Ok(model)
    }

    fn add_pair(&mut self, lexicon: &Lexicon, line: &str) -> Result<(), String> {
        let mut columns = line.split('\t');
        let first = columns.next().unwrap_or("").trim();
        let first = if first == START { first.to_string() } else { normalize(first) };
        let next = normalize(columns.next().unwrap_or("").trim());
        let percent = columns.next().unwrap_or("").trim();

        if first != START && !lexicon.contains(&first) {
            return Err(format!("\"{first}\" is not in words.tsv"));
        }
        if !lexicon.contains(&next) {
            return Err(format!("\"{next}\" is not in words.tsv"));
        }
        let percent: f32 = match percent.parse() {
            Ok(percent) if percent > 0.0 && percent <= 100.0 => percent,
            _ => return Err(format!("\"{percent}\" should be a percent from 0 to 100")),
        };
        let after_first = self.pairs.entry(first.clone()).or_default();
        if after_first.insert(next.clone(), percent / 100.0).is_some() {
            return Err(format!("the pair \"{first}\" + \"{next}\" is listed twice"));
        }
        let listed: f32 = after_first.values().sum();
        if listed > 1.0 {
            return Err(format!("the pairs starting with \"{first}\" add up to more than 100%"));
        }
        Ok(())
    }
}

impl LanguageModel for SeedModel {
    fn log_prob(&self, previous: &[&str], word: &str) -> f32 {
        let before = previous.last().copied().unwrap_or(START);
        let as_pair = self
            .pairs
            .get(before)
            .and_then(|next| next.get(word))
            .copied()
            .unwrap_or(0.0);
        let alone = self.alone.get(word).copied().unwrap_or(UNSEEN);
        (as_pair + BACKOFF * alone).ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORDS: &str = "ចង់\t6000\tjg\nទៅ\t8000\ttv\nទីវី\t500\ttv\n";

    fn model(pairs: &str) -> Result<SeedModel, DataError> {
        let lexicon = Lexicon::from_tsv(WORDS, "", "").unwrap();
        SeedModel::new(&lexicon, pairs)
    }

    #[test]
    fn a_listed_pair_is_more_likely_than_an_unlisted_one() {
        let model = model("ចង់\tទៅ\t12\n").unwrap();
        assert!(model.log_prob(&["ចង់"], "ទៅ") > model.log_prob(&["ចង់"], "ទីវី"));
    }

    #[test]
    fn a_common_word_is_more_likely_than_a_rare_one() {
        let model = model("").unwrap();
        assert!(model.log_prob(&[], "ទៅ") > model.log_prob(&[], "ទីវី"));
    }

    #[test]
    fn the_start_of_a_sentence_can_have_pairs() {
        let model = model("<s>\tចង់\t5\n").unwrap();
        assert!(model.log_prob(&[], "ចង់") > model.log_prob(&["ទៅ"], "ចង់"));
    }

    #[test]
    fn unknown_words_get_a_score_instead_of_a_crash() {
        let model = model("").unwrap();
        assert!(model.log_prob(&["facebook"], "ទៅ").is_finite());
        assert!(model.log_prob(&[], "facebook").is_finite());
    }

    #[test]
    fn bad_pairs_are_reported() {
        let error = |pairs: &str| model(pairs).err().unwrap().message;
        assert!(error("ចង់\tសាលា\t5\n").contains("not in words.tsv"));
        assert!(error("ចង់\tទៅ\tlots\n").contains("percent"));
        assert!(error("ចង់\tទៅ\t5\nចង់\tទៅ\t6\n").contains("twice"));
        assert!(error("ចង់\tទៅ\t60\nចង់\tទីវី\t50\n").contains("100%"));
    }
}
