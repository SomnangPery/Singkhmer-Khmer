use std::collections::HashMap;

use super::lexicon::{data_lines, DataError, Lexicon};
use super::strings::Strings;
use crate::normalize::normalize;

pub trait LanguageModel: Send + Sync {
    fn log_prob(&self, previous: &[&str], word: &str) -> f32;
}

const START: &str = "<s>";
const BACKOFF: f32 = 0.4;
const UNSEEN: f32 = 1e-7;

pub struct SeedModel {
    /// Every word, in sorted order, with its share of all words in `alone`.
    words: Strings,
    alone: Vec<f32>,
    pairs: HashMap<String, HashMap<String, f32>>,
}

impl SeedModel {
    pub fn new(lexicon: &Lexicon, pairs: &str) -> Result<SeedModel, DataError> {
        let total: f32 = lexicon.words().map(|(_, count)| count).sum();
        let mut sorted: Vec<(&str, f32)> = lexicon.words().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        let mut words = Strings::default();
        let mut alone = Vec::with_capacity(sorted.len());
        for (word, count) in sorted {
            words.push(word);
            alone.push(count / total);
        }
        words.shrink_to_fit();
        let mut model = SeedModel {
            words,
            alone,
            pairs: HashMap::new(),
        };
        for (line, text) in data_lines(pairs) {
            model.add_pair(lexicon, text).map_err(|message| DataError {
                file: "pairs",
                line,
                message,
            })?;
        }
        Ok(model)
    }

    fn add_pair(&mut self, lexicon: &Lexicon, line: &str) -> Result<(), String> {
        let mut columns = line.split('\t');
        let first = columns.next().unwrap_or("").trim();
        let first = if first == START {
            first.to_string()
        } else {
            normalize(first)
        };
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
        let alone = self.words.find_sorted(word).map_or(UNSEEN, |at| self.alone[at as usize]);
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
