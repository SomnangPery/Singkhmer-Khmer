//! Picks the most likely sentence for a row of typed tokens.
//!
//! Every token has several candidates, so a sentence of five tokens can have
//! thousands of readings. A beam search goes through the tokens left to right
//! and keeps only the best few partial sentences at each step.

use std::collections::HashSet;

use super::lexicon::Lexicon;
use super::model::LanguageModel;
use super::{Sentence, Word};

/// How many partial sentences survive each step.
const BEAM_WIDTH: usize = 5;
/// How many finished sentences are returned.
const SUGGESTIONS: usize = 3;
/// The weight λ of the language model against the spelling score. To be tuned
/// on test data.
const LANGUAGE_MODEL_WEIGHT: f32 = 1.0;

/// The best sentences for `tokens`, best first. `previous` holds the words
/// already committed before them, for the language model.
pub(crate) fn search(
    lexicon: &Lexicon,
    model: &dyn LanguageModel,
    previous: &[String],
    tokens: &[String],
) -> Vec<Sentence> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut beam = vec![Sentence { words: Vec::new(), score: 0.0 }];
    for token in tokens {
        let candidates = lexicon.candidates(token);
        let mut longer = Vec::with_capacity(beam.len() * candidates.len());
        for sentence in &beam {
            let history: Vec<&str> = previous
                .iter()
                .map(String::as_str)
                .chain(sentence.words.iter().map(|word| word.text.as_str()))
                .collect();
            let history = &history[history.len().saturating_sub(2)..];
            for candidate in &candidates {
                // score = log P(spelling | word) + λ log P(word | words before)
                let score = if candidate.latin {
                    candidate.score
                } else {
                    candidate.score
                        + LANGUAGE_MODEL_WEIGHT * model.log_prob(history, &candidate.text)
                };
                let mut words = sentence.words.clone();
                words.push(Word { text: candidate.text.clone(), latin: candidate.latin });
                longer.push(Sentence { words, score: sentence.score + score });
            }
        }
        // Best first. Equal scores are ordered by text so results never depend
        // on chance.
        longer.sort_by(|a, b| {
            b.score.total_cmp(&a.score).then_with(|| a.text().cmp(&b.text()))
        });
        // Two readings can give the same text (សាលា + រៀន and សាលារៀន): keep one.
        let mut seen = HashSet::new();
        longer.retain(|sentence| seen.insert(sentence.text()));
        longer.truncate(BEAM_WIDTH);
        beam = longer;
    }
    beam.truncate(SUGGESTIONS);
    beam
}
