use crate::chars::*;
use crate::normalize::normalize;
use crate::validate::is_valid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Push {
    Changed,
    Ignored,
    Break,
}

#[derive(Clone, Default)]
struct State {
    base: Option<char>,
    robat: bool,
    coengs: Vec<char>,
    shifter: Option<char>,
    zwnj: bool,
    vowel: Option<char>,
    under_u: bool,
    signs: Vec<char>,
    final_sign: Option<char>,
    pending_coeng: bool,
}

impl State {
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

    fn u_is_meaningful(&self) -> bool {
        is_valid(&normalize(&self.raw(false, true)))
    }

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
            self.vowel = Some(OE);
        } else if is_pair(E, AA) {
            self.vowel = Some(OO);
        } else if c == U {
            self.under_u = true;
            if !self.u_is_meaningful() {
                self.under_u = false;
                self.vowel = Some(U);
            }
        } else if current == U {
            self.vowel = Some(c);
            self.under_u = true;
        } else {
            self.vowel = Some(c);
            self.under_u = false;
        }
        Push::Changed
    }

    fn apply(&mut self, c: char) -> Push {
        let class = class_of(c);

        if self.base.is_none() {
            return match class {
                Class::Base => {
                    self.base = Some(c);
                    Push::Changed
                }
                Class::Other if c != ZWNJ => Push::Break,
                _ => Push::Ignored,
            };
        }

        if self.pending_coeng {
            if class == Class::Coeng {
                return Push::Ignored;
            }
            self.pending_coeng = false;
            if class == Class::Base {
                if self.coengs.len() < 2 {
                    self.coengs.push(c);
                }
                return Push::Changed;
            }
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
            Class::VowelPre | Class::VowelBelow | Class::VowelAbove | Class::VowelPost => self.push_vowel(c),
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

#[derive(Default)]
pub(crate) struct Composer {
    state: State,
    history: Vec<State>,
}

impl Composer {
    pub(crate) fn is_empty(&self) -> bool {
        self.state.base.is_none()
    }

    pub(crate) fn push(&mut self, c: char) -> Push {
        let before = self.state.clone();
        let result = self.state.apply(c);
        if result == Push::Changed && self.state.render().is_some() {
            self.history.push(before);
            return Push::Changed;
        }
        self.state = before;
        if result == Push::Break {
            Push::Break
        } else {
            Push::Ignored
        }
    }

    pub(crate) fn backspace(&mut self) -> bool {
        match self.history.pop() {
            Some(previous) => {
                self.state = previous;
                true
            }
            None => false,
        }
    }

    pub(crate) fn text(&self) -> String {
        self.state.render().unwrap_or_default()
    }

    pub(crate) fn preedit(&self) -> String {
        let mut text = self.text();
        if self.state.pending_coeng {
            text.push(COENG);
        }
        text
    }

    pub(crate) fn take(&mut self) -> String {
        let text = self.text();
        self.clear();
        text
    }

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
