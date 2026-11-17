//! The phrase being typed in romanized mode, one key at a time.
//!
//! The whole phrase stays "composing" until it is finished, so a later word
//! can still change the reading of an earlier one:
//!
//! * a letter adds to the token being typed, which is shown as typed;
//! * Space ends the token, and it turns into the top suggestion's Khmer word;
//! * a second Space, any other character, or picking a suggestion commits the
//!   phrase to the app.

use super::{join, Romanizer, Sentence};
use crate::normalize::normalize;
use crate::validate::is_valid;

/// After this many tokens the phrase is committed on the next Space, so the
/// composing text cannot grow without end.
const MAX_TOKENS: usize = 8;

/// What this engine last wrote into the app, right before the phrase. Decides
/// whether a space is needed in between: Latin words are set off by spaces,
/// Khmer words are not.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tail {
    /// Nothing, or something that already separates words.
    #[default]
    Nothing,
    Khmer,
    Latin,
}

#[derive(Default)]
pub(crate) struct Phrase {
    /// The last two committed words, as context for the language model.
    previous: Vec<String>,
    tail: Tail,
    /// Tokens finished with Space, as typed.
    done: Vec<String>,
    /// The token being typed.
    current: String,
    /// The best sentences for `done` + `current`, best first. Empty only when
    /// nothing is being typed.
    best: Vec<Sentence>,
}

impl Phrase {
    pub(crate) fn is_empty(&self) -> bool {
        self.done.is_empty() && self.current.is_empty()
    }

    /// A letter key.
    pub(crate) fn letter(&mut self, letter: char) {
        self.current.push(letter);
        self.refresh();
    }

    /// The Space key. Returns the text to commit, if any.
    pub(crate) fn space(&mut self) -> String {
        if !self.current.is_empty() {
            // End the token. The tokens are the same as before, so `best` is too.
            self.done.push(std::mem::take(&mut self.current));
            return if self.done.len() >= MAX_TOKENS { self.take() } else { String::new() };
        }
        // Space with no token in progress is a real space: finish the phrase.
        self.other(' ')
    }

    /// Any key that is not a letter, Space or Backspace: finishes the phrase
    /// and passes the character through. Returns the text to commit.
    pub(crate) fn other(&mut self, key: char) -> String {
        let mut text = self.take();
        // A Khmer mark with no consonant to sit on is dropped, as in Khmer
        // mode, so that only valid text ever reaches the app.
        let passed = normalize(&key.to_string());
        if is_valid(&passed) {
            text.push_str(&passed);
        }
        // What follows starts fresh: a new clause, with nothing to space from.
        self.previous.clear();
        self.tail = Tail::Nothing;
        text
    }

    /// The Backspace key: undoes one key. False when nothing is being typed.
    pub(crate) fn backspace(&mut self) -> bool {
        if self.current.is_empty() {
            // Undo the Space that ended the last token.
            return match self.done.pop() {
                Some(token) => {
                    self.current = token;
                    true
                }
                None => false,
            };
        }
        self.current.pop();
        self.refresh();
        true
    }

    /// Commits the top suggestion. Returns the text to commit.
    pub(crate) fn take(&mut self) -> String {
        self.select(0).unwrap_or_default()
    }

    /// Commits suggestion number `index`. None when there is no such suggestion.
    pub(crate) fn select(&mut self, index: usize) -> Option<String> {
        let sentence = self.best.get(index)?.clone();
        let text = format!("{}{}", self.lead(&sentence), sentence.text());

        self.tail = match sentence.words.last() {
            Some(word) if word.latin => Tail::Latin,
            _ => Tail::Khmer,
        };
        self.previous.extend(sentence.words.into_iter().map(|word| word.text));
        let extra = self.previous.len().saturating_sub(2);
        self.previous.drain(..extra);
        self.done.clear();
        self.current.clear();
        self.best.clear();
        // Step 5 of the design: everything goes through the normalizer.
        Some(normalize(&text))
    }

    /// The composing text: finished tokens as Khmer, the token being typed as
    /// the letters typed so far.
    pub(crate) fn preedit(&self) -> String {
        let Some(top) = self.best.first() else {
            return String::new();
        };
        let mut words = top.words.clone();
        if !self.current.is_empty() {
            if let Some(last) = words.last_mut() {
                last.text = self.current.clone();
            }
        }
        format!("{}{}", self.lead(top), join(&words))
    }

    /// The suggestions for the whole phrase, best first.
    pub(crate) fn candidates(&self) -> Vec<String> {
        self.best.iter().map(Sentence::text).collect()
    }

    /// Forgets everything, including the context from earlier phrases.
    pub(crate) fn clear(&mut self) {
        *self = Phrase::default();
    }

    /// The space, if one is needed, between the last committed word and this
    /// sentence.
    fn lead(&self, sentence: &Sentence) -> &'static str {
        let starts_latin = sentence.words.first().is_some_and(|word| word.latin);
        match self.tail {
            Tail::Latin => " ",
            Tail::Khmer if starts_latin => " ",
            _ => "",
        }
    }

    fn refresh(&mut self) {
        let mut tokens = self.done.clone();
        if !self.current.is_empty() {
            tokens.push(self.current.clone());
        }
        self.best = Romanizer::builtin().convert_tokens(&self.previous, &tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Types letters and spaces.
    fn type_keys(phrase: &mut Phrase, keys: &str) -> String {
        let mut committed = String::new();
        for key in keys.chars() {
            if key == ' ' {
                committed.push_str(&phrase.space());
            } else {
                phrase.letter(key);
            }
        }
        committed
    }

    #[test]
    fn the_token_being_typed_is_shown_as_typed() {
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "nh jg");
        assert_eq!(phrase.preedit(), "ខ្ញុំjg");
    }

    #[test]
    fn space_turns_the_token_into_khmer_without_committing() {
        let mut phrase = Phrase::default();
        assert_eq!(type_keys(&mut phrase, "nh jg "), "");
        assert_eq!(phrase.preedit(), "ខ្ញុំចង់");
    }

    #[test]
    fn a_second_space_commits_and_adds_a_real_space() {
        let mut phrase = Phrase::default();
        assert_eq!(type_keys(&mut phrase, "nh jg  "), "ខ្ញុំចង់ ");
        assert!(phrase.is_empty());
        assert_eq!(phrase.preedit(), "");
    }

    #[test]
    fn other_characters_commit_and_pass_through() {
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "soksabay te");
        assert_eq!(phrase.other('?'), "សុខសប្បាយទេ?");
    }

    #[test]
    fn a_khmer_mark_with_no_consonant_is_dropped() {
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "nh");
        assert_eq!(phrase.other('\u{17B6}'), "ខ្ញុំ");
        assert_eq!(phrase.other('\u{17D2}'), "");
    }

    #[test]
    fn backspace_undoes_one_key_including_a_space() {
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "nh jg ");
        assert!(phrase.backspace()); // the space
        assert_eq!(phrase.preedit(), "ខ្ញុំjg");
        assert!(phrase.backspace()); // g
        assert_eq!(phrase.preedit(), "ខ្ញុំj");
        assert!(phrase.backspace()); // j
        assert_eq!(phrase.preedit(), "ខ្ញុំ");
        assert!(phrase.backspace()); // the first space
        assert_eq!(phrase.preedit(), "nh");
        assert!(phrase.backspace());
        assert!(phrase.backspace());
        assert!(!phrase.backspace()); // nothing left
    }

    #[test]
    fn a_long_phrase_is_committed_by_itself() {
        let mut phrase = Phrase::default();
        let committed = type_keys(&mut phrase, "nh jg tv sala rean nh jg tv ");
        assert_eq!(committed, "ខ្ញុំចង់ទៅសាលារៀនខ្ញុំចង់ទៅ");
        assert!(phrase.is_empty());
    }

    #[test]
    fn latin_words_are_set_off_by_spaces_across_commits() {
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "nh leng facebook");
        assert_eq!(phrase.take(), "ខ្ញុំលេង facebook");
        // The next phrase follows a Latin word, so it starts with a space.
        type_keys(&mut phrase, "hz");
        assert_eq!(phrase.preedit(), " hz");
        assert_eq!(phrase.take(), " ហើយ");
        // Khmer after Khmer needs none.
        type_keys(&mut phrase, "nv");
        assert_eq!(phrase.take(), "នៅ");
    }

    #[test]
    fn earlier_words_give_context_to_the_next_phrase() {
        // On its own "nh" is ខ្ញុំ (I); straight after ចង់ (want) it is ញ៉ាំ (eat).
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "nh");
        assert_eq!(phrase.take(), "ខ្ញុំ");
        phrase.clear();
        type_keys(&mut phrase, "jg");
        phrase.take();
        type_keys(&mut phrase, "nh");
        assert_eq!(phrase.take(), "ញ៉ាំ");
    }

    #[test]
    fn selecting_another_suggestion_commits_that_one() {
        let mut phrase = Phrase::default();
        type_keys(&mut phrase, "nh");
        let candidates = phrase.candidates();
        assert!(candidates.len() >= 2);
        assert_eq!(phrase.select(1), Some(candidates[1].clone()));
        assert!(phrase.is_empty());
        assert_eq!(phrase.select(0), None);
    }
}
