//! Khmer mode: building one syllable from key presses.
//!
//! The [`Composer`] holds the syllable being typed. Each key either changes
//! it, is ignored, or does not belong to it (which tells the engine to finish
//! this syllable and start again). Whatever the keys and whatever their order,
//! the text the composer hands back is normalized and valid: a key that would
//! leave no valid syllable is ignored.

use crate::chars::*;
use crate::normalize::normalize;
use crate::validate::is_valid;

/// What a key press did to the syllable being typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Push {
    /// The key was taken into the syllable.
    Changed,
    /// The key has no valid meaning here and was dropped.
    Ignored,
    /// The key is not part of this syllable. The caller should take the
    /// finished syllable and then deal with the key.
    Break,
}

/// The parts of the syllable as the user has typed them so far.
#[derive(Clone, Default)]
struct State {
    base: Option<char>,
    robat: bool,
    /// The consonants of the subscripts, in typing order.
    coengs: Vec<char>,
    shifter: Option<char>,
    /// The user asked for a ZWNJ after the shifter.
    zwnj: bool,
    vowel: Option<char>,
    /// The user typed ុ as well as `vowel`. It is shown only where it has a
    /// valid meaning: as a shifter drawn low (ស + ុ + ី is ស៊ី).
    under_u: bool,
    signs: Vec<char>,
    final_sign: Option<char>,
    /// The coeng sign was typed and its consonant has not arrived yet.
    pending_coeng: bool,
}

impl State {
    /// The parts in typing-slot order, before normalization.
    fn raw(&self, with_zwnj: bool, with_u: bool) -> String {
        let Some(base) = self.base else {
            return String::new();
        };
        let mut s = String::new();
        s.push(base);
        if self.robat {
            s.push(ROBAT);
        }
        for &c in &self.coengs {
            s.push(COENG);
            s.push(c);
        }
        if let Some(shifter) = self.shifter {
            s.push(shifter);
            if with_zwnj {
                s.push(ZWNJ);
            }
        }
        if with_u {
            s.push(U);
        }
        s.extend(self.vowel);
        s.extend(&self.signs);
        s.extend(self.final_sign);
        s
    }

    /// The best valid text for these parts, or `None` if there is none.
    ///
    /// The optional parts (ZWNJ, the extra ុ) are included when the result is
    /// valid with them and left out when it is not.
    fn render(&self) -> Option<String> {
        if self.base.is_none() {
            return Some(String::new());
        }
        let zwnj_choices: &[bool] = if self.zwnj { &[true, false] } else { &[false] };
        let u_choices: &[bool] = if self.under_u { &[true, false] } else { &[false] };
        for &with_zwnj in zwnj_choices {
            for &with_u in u_choices {
                let text = normalize(&self.raw(with_zwnj, with_u));
                if is_valid(&text) {
                    return Some(text);
                }
            }
        }
        None
    }

    /// True if the extra ុ has a valid meaning right now.
    fn u_is_meaningful(&self) -> bool {
        is_valid(&normalize(&self.raw(false, true)))
    }

    /// A vowel key, when the syllable has a base.
    fn push_vowel(&mut self, c: char) -> Push {
        let Some(current) = self.vowel else {
            self.vowel = Some(c);
            return Push::Changed;
        };
        if current == c {
            return Push::Ignored;
        }
        let is_pair = |a: char, b: char| (current == a && c == b) || (current == b && c == a);
        if is_pair(E, II) {
            // េ + ី looks like ើ
            self.vowel = Some(OE);
        } else if is_pair(E, AA) {
            // េ + ា looks like ោ
            self.vowel = Some(OO);
        } else if c == U {
            // ុ typed onto another vowel. Keep both only if the ុ has a valid
            // meaning (a shifter drawn low); otherwise ុ replaces the vowel.
            self.under_u = true;
            if !self.u_is_meaningful() {
                self.under_u = false;
                self.vowel = Some(U);
            }
        } else if current == U {
            // Another vowel typed onto ុ. Remember the ុ: if ំ arrives next
            // (ញ + ុ + ា + ំ) the pair turns out to be a shifter after all.
            self.vowel = Some(c);
            self.under_u = true;
        } else {
            // Any other second vowel replaces the first.
            self.vowel = Some(c);
            self.under_u = false;
        }
        Push::Changed
    }

    /// Applies one key. May leave the state changed even when the key is
    /// ignored; [`Composer::push`] restores it in that case.
    fn apply(&mut self, c: char) -> Push {
        let class = class_of(c);

        if self.base.is_none() {
            return match class {
                Class::Base => {
                    self.base = Some(c);
                    Push::Changed
                }
                Class::Other if c != ZWNJ => Push::Break,
                // A vowel, sign or coeng with no consonant to attach to.
                _ => Push::Ignored,
            };
        }

        if self.pending_coeng {
            if class == Class::Coeng {
                return Push::Ignored;
            }
            self.pending_coeng = false;
            if class == Class::Base {
                // The consonant of the subscript. A third subscript is dropped.
                if self.coengs.len() < 2 {
                    self.coengs.push(c);
                }
                return Push::Changed;
            }
            // Anything else cancels the coeng sign and is handled normally.
        }

        match class {
            Class::Base => Push::Break,
            Class::Coeng => {
                self.pending_coeng = true;
                Push::Changed
            }
            Class::Robat => {
                if self.robat {
                    Push::Ignored
                } else {
                    self.robat = true;
                    Push::Changed
                }
            }
            Class::Shifter => {
                if self.shifter == Some(c) {
                    Push::Ignored
                } else {
                    self.shifter = Some(c);
                    Push::Changed
                }
            }
            Class::VowelPre | Class::VowelBelow | Class::VowelAbove | Class::VowelPost => {
                self.push_vowel(c)
            }
            Class::Sign => {
                if self.signs.contains(&c) || self.signs.len() >= 2 {
                    Push::Ignored
                } else {
                    self.signs.push(c);
                    Push::Changed
                }
            }
            Class::Final => {
                if self.final_sign == Some(c) {
                    Push::Ignored
                } else {
                    self.final_sign = Some(c);
                    Push::Changed
                }
            }
            Class::Other => {
                if c != ZWNJ {
                    Push::Break
                } else if self.shifter.is_none() || self.zwnj {
                    Push::Ignored
                } else {
                    self.zwnj = true;
                    Push::Changed
                }
            }
        }
    }
}

/// The syllable being typed, with undo history for backspace.
#[derive(Default)]
pub(crate) struct Composer {
    state: State,
    /// The state before each accepted key, oldest first.
    history: Vec<State>,
}

impl Composer {
    /// True when no syllable is being typed.
    pub(crate) fn is_empty(&self) -> bool {
        self.state.base.is_none()
    }

    /// Feeds one key to the syllable.
    pub(crate) fn push(&mut self, c: char) -> Push {
        let before = self.state.clone();
        let result = self.state.apply(c);
        if result == Push::Changed && self.state.render().is_some() {
            self.history.push(before);
            return Push::Changed;
        }
        // Ignored, not part of the syllable, or no valid syllable would be
        // left: put everything back as it was.
        self.state = before;
        if result == Push::Break {
            Push::Break
        } else {
            Push::Ignored
        }
    }

    /// Undoes the last accepted key. Returns false if there was nothing to undo.
    pub(crate) fn backspace(&mut self) -> bool {
        match self.history.pop() {
            Some(previous) => {
                self.state = previous;
                true
            }
            None => false,
        }
    }

    /// The syllable as it will be committed: normalized and valid.
    pub(crate) fn text(&self) -> String {
        self.state.render().unwrap_or_default()
    }

    /// The syllable as shown while typing. Same as [`Composer::text`], plus the
    /// coeng sign if the user has typed it and its consonant is still to come.
    pub(crate) fn preedit(&self) -> String {
        let mut text = self.text();
        if self.state.pending_coeng {
            text.push(COENG);
        }
        text
    }

    /// Takes the finished syllable and starts afresh.
    pub(crate) fn take(&mut self) -> String {
        let text = self.text();
        self.clear();
        text
    }

    /// Throws away the syllable being typed.
    pub(crate) fn clear(&mut self) {
        self.state = State::default();
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(keys: &str) -> String {
        let mut composer = Composer::default();
        for key in keys.chars() {
            composer.push(key);
        }
        composer.text()
    }

    #[test]
    fn any_typing_order_gives_the_same_syllable() {
        let stri = "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}";
        assert_eq!(typed("\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"), stri);
        assert_eq!(typed("\u{179F}\u{17B8}\u{17D2}\u{179A}\u{17D2}\u{178F}"), stri);
        assert_eq!(typed("\u{179F}\u{17D2}\u{179A}\u{17B8}\u{17D2}\u{178F}"), stri);
    }

    #[test]
    fn second_vowel_replaces_the_first() {
        assert_eq!(typed("\u{1780}\u{17B6}\u{17B8}"), "\u{1780}\u{17B8}");
    }

    #[test]
    fn lookalike_vowel_pairs_merge() {
        assert_eq!(typed("\u{1780}\u{17C1}\u{17B8}"), "\u{1780}\u{17BE}");
        assert_eq!(typed("\u{1780}\u{17B6}\u{17C1}"), "\u{1780}\u{17C4}");
    }

    #[test]
    fn u_with_an_above_vowel_becomes_a_shifter() {
        assert_eq!(typed("\u{179F}\u{17BB}\u{17B8}"), "\u{179F}\u{17CA}\u{17B8}");
        // ញ + ុ + ា + ំ : the ុ only makes sense once the ំ arrives.
        assert_eq!(typed("\u{1789}\u{17BB}\u{17B6}"), "\u{1789}\u{17B6}");
        assert_eq!(
            typed("\u{1789}\u{17BB}\u{17B6}\u{17C6}"),
            "\u{1789}\u{17C9}\u{17B6}\u{17C6}"
        );
    }

    #[test]
    fn marks_with_no_base_are_ignored() {
        let mut composer = Composer::default();
        assert_eq!(composer.push('\u{17B6}'), Push::Ignored);
        assert_eq!(composer.push(COENG), Push::Ignored);
        assert!(composer.is_empty());
    }

    #[test]
    fn repeated_signs_are_dropped() {
        let mut composer = Composer::default();
        composer.push('\u{1780}');
        assert_eq!(composer.push(NIKAHIT), Push::Changed);
        assert_eq!(composer.push(NIKAHIT), Push::Ignored);
        assert_eq!(composer.text(), "\u{1780}\u{17C6}");
    }

    #[test]
    fn pending_coeng_shows_while_typing_but_is_never_committed() {
        let mut composer = Composer::default();
        composer.push('\u{1780}');
        composer.push(COENG);
        assert_eq!(composer.preedit(), "\u{1780}\u{17D2}");
        assert_eq!(composer.text(), "\u{1780}");
    }

    #[test]
    fn a_new_consonant_or_other_character_ends_the_syllable() {
        let mut composer = Composer::default();
        composer.push('\u{1780}');
        assert_eq!(composer.push('\u{1781}'), Push::Break);
        assert_eq!(composer.push(' '), Push::Break);
        assert_eq!(composer.take(), "\u{1780}");
        assert!(composer.is_empty());
    }

    #[test]
    fn backspace_undoes_one_key() {
        let mut composer = Composer::default();
        for key in "\u{1780}\u{17B6}\u{17C6}".chars() {
            composer.push(key);
        }
        assert!(composer.backspace());
        assert_eq!(composer.text(), "\u{1780}\u{17B6}");
        assert!(composer.backspace());
        assert!(composer.backspace());
        assert!(composer.is_empty());
        assert!(!composer.backspace());
    }
}
