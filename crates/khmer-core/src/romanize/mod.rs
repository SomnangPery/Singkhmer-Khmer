//! Romanized mode: Khmer typed with Latin letters ("nh jg tv" → ខ្ញុំចង់ទៅ).
//!
//! The steps, as in the design document:
//!
//! 1. The input is split into tokens at spaces.
//! 2. Each token gets candidates from the romanization dictionary
//!    ([`Lexicon`]): listed spellings, spellings that differ by a variant rule,
//!    and longer spellings that start with the token.
//! 3. A beam search scores whole sentences, using the spelling fit and a
//!    [`LanguageModel`], and keeps the best ones.
//! 4. A token with no good Khmer match stays in Latin letters.
//!
//! [`Romanizer::convert`] does this for a whole line of text. The keyboard
//! itself goes through [`crate::Engine`] in [`crate::Mode::Romanized`].

mod lexicon;
mod model;
mod phrase;
mod search;
mod skeleton;

use std::sync::OnceLock;

pub use lexicon::{DataError, Lexicon};
pub use model::{LanguageModel, SeedModel};
pub(crate) use phrase::Phrase;

const WORDS: &str = include_str!("../../data/words.tsv");
const IMPORTED: &str = include_str!("../../data/words-imported.tsv");
const PAIRS: &str = include_str!("../../data/pairs.tsv");
const ENGLISH: &str = include_str!("../../data/english.txt");

/// One word of a converted sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// The Khmer word, or the typed letters when `latin` is true.
    pub text: String,
    /// True when the token was kept in Latin letters.
    pub latin: bool,
}

/// One possible reading of the typed tokens.
#[derive(Clone, Debug, PartialEq)]
pub struct Sentence {
    /// One word per typed token.
    pub words: Vec<Word>,
    /// How likely this reading is, as a logarithm: closer to zero is better.
    pub score: f32,
}

impl Sentence {
    /// The sentence as text. Khmer words are written without spaces between
    /// them; a word kept in Latin letters is set off by spaces.
    pub fn text(&self) -> String {
        join(&self.words)
    }
}

pub(crate) fn join(words: &[Word]) -> String {
    let mut text = String::new();
    for (index, word) in words.iter().enumerate() {
        if index > 0 && (word.latin || words[index - 1].latin) {
            text.push(' ');
        }
        text.push_str(&word.text);
    }
    text
}

/// Converts romanized Khmer into Khmer script.
pub struct Romanizer {
    lexicon: Lexicon,
    model: Box<dyn LanguageModel>,
}

impl Romanizer {
    /// A converter with the given dictionary and language model.
    pub fn new(lexicon: Lexicon, model: Box<dyn LanguageModel>) -> Romanizer {
        Romanizer { lexicon, model }
    }

    /// A converter built from the text of the data files, with the
    /// hand-written [`SeedModel`] as its language model. See
    /// [`Lexicon::from_tsv`] for what `words` and `imported` are.
    pub fn from_data(
        words: &str,
        imported: &str,
        pairs: &str,
        english: &str,
    ) -> Result<Romanizer, DataError> {
        let lexicon = Lexicon::from_tsv(words, imported, english)?;
        let model = SeedModel::new(&lexicon, pairs)?;
        Ok(Romanizer::new(lexicon, Box::new(model)))
    }

    /// The converter with the data files that are built into the library.
    pub fn builtin() -> &'static Romanizer {
        static BUILTIN: OnceLock<Romanizer> = OnceLock::new();
        BUILTIN.get_or_init(|| {
            // The built-in files are checked by the tests, so this cannot fail
            // in a build that passed them.
            Romanizer::from_data(WORDS, IMPORTED, PAIRS, ENGLISH)
                .unwrap_or_else(|error| panic!("built-in romanization data: {error}"))
        })
    }

    /// The dictionary in use.
    pub fn lexicon(&self) -> &Lexicon {
        &self.lexicon
    }

    /// Converts a line of romanized text. Returns up to three readings, best
    /// first; empty only when `input` has no tokens.
    pub fn convert(&self, input: &str) -> Vec<Sentence> {
        let tokens: Vec<String> = input.split_whitespace().map(str::to_string).collect();
        self.convert_tokens(&[], &tokens)
    }

    /// Like [`Romanizer::convert`], for input already split into tokens.
    /// `previous` holds the words that come before them, as context; pass an
    /// empty slice at the start of a sentence.
    pub fn convert_tokens(&self, previous: &[String], tokens: &[String]) -> Vec<Sentence> {
        search::search(&self.lexicon, self.model.as_ref(), previous, tokens)
    }
}
