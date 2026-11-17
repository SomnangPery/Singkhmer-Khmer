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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub latin: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sentence {
    pub words: Vec<Word>,
    pub score: f32,
}

impl Sentence {
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

pub struct Romanizer {
    lexicon: Lexicon,
    model: Box<dyn LanguageModel>,
}

impl Romanizer {
    pub fn new(lexicon: Lexicon, model: Box<dyn LanguageModel>) -> Romanizer {
        Romanizer { lexicon, model }
    }

    pub fn from_data(words: &str, imported: &str, pairs: &str, english: &str) -> Result<Romanizer, DataError> {
        let lexicon = Lexicon::from_tsv(words, imported, english)?;
        let model = SeedModel::new(&lexicon, pairs)?;
        Ok(Romanizer::new(lexicon, Box::new(model)))
    }

    pub fn builtin() -> &'static Romanizer {
        static BUILTIN: OnceLock<Romanizer> = OnceLock::new();
        BUILTIN.get_or_init(|| {
            Romanizer::from_data(WORDS, IMPORTED, PAIRS, ENGLISH)
                .unwrap_or_else(|error| panic!("built-in romanization data: {error}"))
        })
    }

    pub fn lexicon(&self) -> &Lexicon {
        &self.lexicon
    }

    pub fn convert(&self, input: &str) -> Vec<Sentence> {
        let tokens: Vec<String> = input.split_whitespace().map(str::to_string).collect();
        self.convert_tokens(&[], &tokens)
    }

    pub fn convert_tokens(&self, previous: &[String], tokens: &[String]) -> Vec<Sentence> {
        search::search(&self.lexicon, self.model.as_ref(), previous, tokens)
    }
}
