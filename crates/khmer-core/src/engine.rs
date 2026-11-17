use crate::chars::is_base;
use crate::compose::{Composer, Push};
use crate::normalize::normalize;
use crate::romanize::Phrase;

pub const BACKSPACE: char = '\u{8}';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Khmer,
    Romanized,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub commit: String,
    pub preedit: String,
    pub handled: bool,
}

#[derive(Default)]
pub struct Engine {
    mode: Mode,
    composer: Composer,
    phrase: Phrase,
}

impl Engine {
    pub fn new() -> Self {
        Engine::default()
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) -> Output {
        let output = self.commit();
        self.mode = mode;
        output
    }

    pub fn on_key(&mut self, key: char) -> Output {
        match self.mode {
            Mode::Khmer => self.on_khmer_key(key),
            Mode::Romanized => self.on_romanized_key(key),
        }
    }

    pub fn preedit(&self) -> String {
        match self.mode {
            Mode::Khmer => self.composer.preedit(),
            Mode::Romanized => self.phrase.preedit(),
        }
    }

    pub fn is_composing(&self) -> bool {
        !self.composer.is_empty() || !self.phrase.is_empty()
    }

    pub fn candidates(&self) -> Vec<String> {
        match self.mode {
            Mode::Khmer => Vec::new(),
            Mode::Romanized => self.phrase.candidates(),
        }
    }

    pub fn select(&mut self, index: usize) -> Output {
        let text = match self.mode {
            Mode::Khmer => String::new(),
            Mode::Romanized => self.phrase.select(index).unwrap_or_default(),
        };
        self.output(text, true)
    }

    pub fn commit(&mut self) -> Output {
        let text = match self.mode {
            Mode::Khmer => self.composer.take(),
            Mode::Romanized => self.phrase.take(),
        };
        self.output(text, true)
    }

    pub fn reset(&mut self) {
        self.composer.clear();
        self.phrase.clear();
    }

    fn on_khmer_key(&mut self, key: char) -> Output {
        if key == BACKSPACE {
            let handled = self.composer.backspace();
            return self.output(String::new(), handled);
        }
        match self.composer.push(key) {
            Push::Changed | Push::Ignored => self.output(String::new(), true),
            Push::Break => {
                let mut commit = self.composer.take();
                if is_base(key) {
                    self.composer.push(key);
                } else {
                    commit.push_str(&normalize(&key.to_string()));
                }
                self.output(commit, true)
            }
        }
    }

    fn on_romanized_key(&mut self, key: char) -> Output {
        if key == BACKSPACE {
            let handled = self.phrase.backspace();
            return self.output(String::new(), handled);
        }
        let commit = if key.is_ascii_alphabetic() {
            self.phrase.letter(key);
            String::new()
        } else if key == ' ' {
            self.phrase.space()
        } else {
            self.phrase.other(key)
        };
        self.output(commit, true)
    }

    fn output(&self, commit: String, handled: bool) -> Output {
        Output {
            commit,
            preedit: self.preedit(),
            handled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_all(engine: &mut Engine, keys: &str) -> String {
        let mut app = String::new();
        for key in keys.chars() {
            app.push_str(&engine.on_key(key).commit);
        }
        app.push_str(&engine.commit().commit);
        app
    }

    #[test]
    fn design_doc_example() {
        let mut engine = Engine::new();
        assert_eq!(
            type_all(&mut engine, "\u{179F}\u{17B8}\u{17D2}\u{179A}\u{17D2}\u{178F}"),
            "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"
        );
    }

    #[test]
    fn syllable_is_committed_when_the_next_one_starts() {
        let mut engine = Engine::new();
        let first = engine.on_key('\u{1780}');
        assert_eq!(first.commit, "");
        assert_eq!(first.preedit, "\u{1780}");

        let second = engine.on_key('\u{1781}');
        assert_eq!(second.commit, "\u{1780}");
        assert_eq!(second.preedit, "\u{1781}");
    }

    #[test]
    fn space_commits_the_syllable_and_passes_through() {
        let mut engine = Engine::new();
        engine.on_key('\u{1780}');
        engine.on_key('\u{17B6}');
        let output = engine.on_key(' ');
        assert_eq!(output.commit, "\u{1780}\u{17B6} ");
        assert_eq!(output.preedit, "");
    }

    #[test]
    fn backspace_is_ours_only_while_typing_a_syllable() {
        let mut engine = Engine::new();
        assert!(!engine.on_key(BACKSPACE).handled);

        engine.on_key('\u{1780}');
        engine.on_key('\u{17B6}');
        let output = engine.on_key(BACKSPACE);
        assert!(output.handled);
        assert_eq!(output.preedit, "\u{1780}");
    }

    #[test]
    fn pending_coeng_is_shown_but_not_committed() {
        let mut engine = Engine::new();
        engine.on_key('\u{1780}');
        assert_eq!(engine.on_key('\u{17D2}').preedit, "\u{1780}\u{17D2}");
        assert_eq!(engine.commit().commit, "\u{1780}");
    }

    #[test]
    fn switching_mode_commits_what_was_being_typed() {
        let mut engine = Engine::new();
        engine.on_key('\u{1780}');
        let output = engine.set_mode(Mode::Romanized);
        assert_eq!(output.commit, "\u{1780}");
        assert_eq!(engine.mode(), Mode::Romanized);

        engine.on_key('n');
        engine.on_key('h');
        let back = engine.set_mode(Mode::Khmer);
        assert_eq!(back.commit, "ខ្ញុំ");
        assert_eq!(back.preedit, "");
    }

    fn romanized() -> Engine {
        let mut engine = Engine::new();
        engine.set_mode(Mode::Romanized);
        engine
    }

    #[test]
    fn romanized_design_doc_example() {
        let mut engine = romanized();
        let mut app = String::new();
        for key in "nh jg tv sala rean".chars() {
            let output = engine.on_key(key);
            assert!(output.handled);
            app.push_str(&output.commit);
        }
        assert_eq!(app, "");
        assert_eq!(engine.preedit(), "ខ្ញុំចង់ទៅសាលាrean");
        assert_eq!(engine.candidates()[0], "ខ្ញុំចង់ទៅសាលារៀន");
        assert_eq!(engine.commit().commit, "ខ្ញុំចង់ទៅសាលារៀន");
        assert!(!engine.is_composing());
    }

    #[test]
    fn romanized_select_commits_the_chosen_suggestion() {
        let mut engine = romanized();
        engine.on_key('t');
        engine.on_key('v');
        let candidates = engine.candidates();
        assert_eq!(candidates[0], "ទៅ");
        let tv = candidates.iter().position(|c| c == "ទីវី").expect("ទីវី is offered");
        let output = engine.select(tv);
        assert_eq!(output.commit, "ទីវី");
        assert_eq!(output.preedit, "");
        assert!(engine.candidates().is_empty());
    }

    #[test]
    fn romanized_punctuation_commits_the_phrase() {
        let mut engine = romanized();
        for key in "orkun jren".chars() {
            engine.on_key(key);
        }
        assert_eq!(engine.on_key('!').commit, "អរគុណច្រើន!");
    }

    #[test]
    fn romanized_backspace_is_ours_only_while_typing() {
        let mut engine = romanized();
        assert!(!engine.on_key(BACKSPACE).handled);
        engine.on_key('n');
        let output = engine.on_key(BACKSPACE);
        assert!(output.handled);
        assert_eq!(output.preedit, "");
        assert!(!engine.on_key(BACKSPACE).handled);
    }

    #[test]
    fn khmer_mode_has_no_suggestions_yet() {
        let mut engine = Engine::new();
        engine.on_key('\u{1780}');
        assert!(engine.candidates().is_empty());
        assert_eq!(engine.select(0).commit, "");
        assert_eq!(engine.preedit(), "\u{1780}");
    }

    #[test]
    fn reset_discards_without_committing() {
        let mut engine = Engine::new();
        engine.on_key('\u{1780}');
        engine.reset();
        assert_eq!(engine.preedit(), "");
        assert_eq!(engine.commit().commit, "");
    }
}
