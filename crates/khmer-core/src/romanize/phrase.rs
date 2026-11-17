use super::{join, Romanizer, Sentence};
use crate::normalize::normalize;
use crate::validate::is_valid;

const MAX_TOKENS: usize = 8;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tail {
    #[default]
    Nothing,
    Khmer,
    Latin,
}

#[derive(Default)]
pub(crate) struct Phrase {
    previous: Vec<String>,
    tail: Tail,
    done: Vec<String>,
    current: String,
    best: Vec<Sentence>,
}

impl Phrase {
    pub(crate) fn is_empty(&self) -> bool {
        self.done.is_empty() && self.current.is_empty()
    }

    pub(crate) fn letter(&mut self, letter: char) {
        self.current.push(letter);
        self.refresh();
    }

    pub(crate) fn space(&mut self) -> String {
        if !self.current.is_empty() {
            self.done.push(std::mem::take(&mut self.current));
            return if self.done.len() >= MAX_TOKENS {
                self.take()
            } else {
                String::new()
            };
        }
        self.other(' ')
    }

    pub(crate) fn other(&mut self, key: char) -> String {
        let mut text = self.take();
        let passed = normalize(&key.to_string());
        if is_valid(&passed) {
            text.push_str(&passed);
        }
        self.previous.clear();
        self.tail = Tail::Nothing;
        text
    }

    pub(crate) fn backspace(&mut self) -> bool {
        if self.current.is_empty() {
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

    pub(crate) fn take(&mut self) -> String {
        self.select(0).unwrap_or_default()
    }

    /// The words that suggestion `index` has where the top suggestion has
    /// another, each with the spelling typed for it: what the user corrects
    /// by picking that suggestion. Empty for the top suggestion.
    pub(crate) fn corrections(&self, index: usize) -> Vec<(String, String)> {
        let (Some(top), Some(chosen)) = (self.best.first(), self.best.get(index)) else {
            return Vec::new();
        };
        if index == 0 {
            return Vec::new();
        }
        let mut tokens = self.done.clone();
        if !self.current.is_empty() {
            tokens.push(self.current.clone());
        }
        chosen
            .words
            .iter()
            .zip(&tokens)
            .enumerate()
            .filter(|(at, (word, _))| !word.latin && top.words.get(*at).is_some_and(|other| other.text != word.text))
            .map(|(_, (word, token))| (token.to_ascii_lowercase(), word.text.clone()))
            .collect()
    }

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
        Some(normalize(&text))
    }

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

    pub(crate) fn typed(&self) -> String {
        let mut tokens = self.done.clone();
        if !self.current.is_empty() {
            tokens.push(self.current.clone());
        }
        tokens.join(" ")
    }

    pub(crate) fn candidates(&self) -> Vec<String> {
        self.best.iter().map(Sentence::text).collect()
    }

    pub(crate) fn clear(&mut self) {
        *self = Phrase::default();
    }

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
        assert!(phrase.backspace());
        assert_eq!(phrase.preedit(), "ខ្ញុំjg");
        assert!(phrase.backspace());
        assert_eq!(phrase.preedit(), "ខ្ញុំj");
        assert!(phrase.backspace());
        assert_eq!(phrase.preedit(), "ខ្ញុំ");
        assert!(phrase.backspace());
        assert_eq!(phrase.preedit(), "nh");
        assert!(phrase.backspace());
        assert!(phrase.backspace());
        assert!(!phrase.backspace());
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
        type_keys(&mut phrase, "hz");
        assert_eq!(phrase.preedit(), " hz");
        assert_eq!(phrase.take(), " ហើយ");
        type_keys(&mut phrase, "nv");
        assert_eq!(phrase.take(), "នៅ");
    }

    #[test]
    fn earlier_words_give_context_to_the_next_phrase() {
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
