use std::collections::HashSet;

use super::lexicon::Lexicon;
use super::model::LanguageModel;
use super::{user, Sentence, Word};

const BEAM_WIDTH: usize = 5;
const SUGGESTIONS: usize = 3;
const LANGUAGE_MODEL_WEIGHT: f32 = 1.0;

pub(crate) fn search(
    lexicon: &Lexicon,
    model: &dyn LanguageModel,
    previous: &[String],
    tokens: &[String],
) -> Vec<Sentence> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut beam = vec![Sentence {
        words: Vec::new(),
        score: 0.0,
    }];
    for token in tokens {
        let mut candidates = user::candidates(token);
        candidates.extend(lexicon.candidates(token));
        let mut longer = Vec::with_capacity(beam.len() * candidates.len());
        for sentence in &beam {
            let history: Vec<&str> = previous
                .iter()
                .map(String::as_str)
                .chain(sentence.words.iter().map(|word| word.text.as_str()))
                .collect();
            let history = &history[history.len().saturating_sub(2)..];
            for candidate in &candidates {
                let score = if candidate.fixed {
                    candidate.score
                } else {
                    candidate.score + LANGUAGE_MODEL_WEIGHT * model.log_prob(history, &candidate.text)
                };
                let mut words = sentence.words.clone();
                words.push(Word {
                    text: candidate.text.clone(),
                    latin: candidate.latin,
                });
                longer.push(Sentence {
                    words,
                    score: sentence.score + score,
                });
            }
        }
        longer.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.text().cmp(&b.text())));
        let mut seen = HashSet::new();
        longer.retain(|sentence| seen.insert(sentence.text()));
        longer.truncate(BEAM_WIDTH);
        beam = longer;
    }
    beam.truncate(SUGGESTIONS);
    beam
}
