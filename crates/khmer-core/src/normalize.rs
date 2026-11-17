//! The normalizer: one stored form for every Khmer syllable.
//!
//! [`normalize`] never changes how correctly typed text looks. It only changes
//! which code points are stored, and in what order, so that the same word is
//! always stored the same way. Text it cannot make sense of (a vowel with no
//! consonant, a coeng sign with nothing after it) is passed through untouched
//! for [`crate::validate()`] to report.

use crate::chars::*;

/// Choices a caller can make about normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Store subscript ដ (coeng da) as subscript ត (coeng ta).
    ///
    /// The two look identical in Modern Khmer and people type the wrong one all
    /// the time, so UTN #61 says keyboards should store only coeng ta. On by
    /// default. Turn it off for Old or Middle Khmer text, where they differ.
    pub fold_coeng_da: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { fold_coeng_da: true }
    }
}

/// Normalizes `text` with the default [`Options`].
///
/// ```
/// // ស្ត្រី typed as: ស, vowel ី, subscript រ, subscript ត
/// let typed = "\u{179F}\u{17B8}\u{17D2}\u{179A}\u{17D2}\u{178F}";
/// let stored = "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}";
/// assert_eq!(khmer_core::normalize(typed), stored);
/// ```
pub fn normalize(text: &str) -> String {
    normalize_with(text, Options::default())
}

/// Normalizes `text` with the given options.
pub fn normalize_with(text: &str, options: Options) -> String {
    let chars = prepass(text);
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !is_base(c) {
            out.push(c);
            i += 1;
            // A subscript with no base before it. Keep the pair together and
            // untouched, so its consonant is not mistaken for a syllable start.
            if c == COENG && i < chars.len() && is_base(chars[i]) {
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        let (mut syllable, end) = Syllable::read(&chars, i);
        syllable.fix(options);
        syllable.write(&mut out);
        i = end;
    }
    out
}

/// Character-level fixes that do not depend on the syllable.
fn prepass(text: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match c {
            // The deprecated letters ឣ and ឤ: Unicode says to use អ and អា.
            '\u{17A3}' => out.push('\u{17A2}'),
            '\u{17A4}' => {
                out.push('\u{17A2}');
                out.push(AA);
            }
            // A doubled coeng sign is a slip of the finger: keep one.
            COENG if out.last() == Some(&COENG) => {}
            _ => out.push(c),
        }
    }
    out
}

/// One syllable taken apart: a base and everything attached to it, sorted
/// into the slot each character belongs in. Within a slot, characters keep
/// the order they were typed in.
#[derive(Default)]
struct Syllable {
    base: char,
    robats: Vec<char>,
    /// The consonants of the subscripts, without their coeng signs.
    coengs: Vec<char>,
    shifters: Vec<char>,
    /// How many ZWNJ belong to the shifter.
    zwnj: usize,
    pre: Vec<char>,
    below: Vec<char>,
    above: Vec<char>,
    post: Vec<char>,
    signs: Vec<char>,
    finals: Vec<char>,
}

impl Syllable {
    /// Reads the syllable whose base is `chars[start]`. Returns it together
    /// with the index of the first character that is not part of it.
    fn read(chars: &[char], start: usize) -> (Syllable, usize) {
        let mut s = Syllable {
            base: chars[start],
            ..Syllable::default()
        };
        let mut j = start + 1;
        while j < chars.len() {
            let c = chars[j];
            let next = chars.get(j + 1).copied();
            match class_of(c) {
                // The next syllable starts here.
                Class::Base => break,
                Class::Coeng => match next {
                    Some(sub) if is_base(sub) => {
                        s.coengs.push(sub);
                        j += 2;
                        continue;
                    }
                    // A coeng sign with no consonant after it ends the syllable.
                    _ => break,
                },
                Class::Other => {
                    // A ZWNJ belongs to the syllable only when it is attached to
                    // a shifter: just before one, or after one while we are still
                    // in the consonant cluster. Any other ZWNJ is left alone.
                    let before_shifter = next.map_or(false, |n| class_of(n) == Class::Shifter);
                    let after_shifter = !s.shifters.is_empty() && !s.has_vowel_or_sign();
                    if c == ZWNJ && (before_shifter || after_shifter) {
                        s.zwnj += 1;
                    } else {
                        break;
                    }
                }
                Class::Robat => s.robats.push(c),
                Class::Shifter => s.shifters.push(c),
                Class::VowelPre => s.pre.push(c),
                Class::VowelBelow => s.below.push(c),
                Class::VowelAbove => s.above.push(c),
                Class::VowelPost => s.post.push(c),
                Class::Sign => s.signs.push(c),
                Class::Final => s.finals.push(c),
            }
            j += 1;
        }
        (s, j)
    }

    fn has_vowel_or_sign(&self) -> bool {
        !(self.pre.is_empty()
            && self.below.is_empty()
            && self.above.is_empty()
            && self.post.is_empty()
            && self.signs.is_empty()
            && self.finals.is_empty())
    }

    /// Applies the rules that sorting into slots cannot express.
    fn fix(&mut self, options: Options) {
        // Rule: subscript រ is stored after any other subscript.
        // (A stable sort: `false` sorts before `true`, everything else keeps its order.)
        self.coengs.sort_by_key(|&c| c == RO);

        // Rule: subscript ដ is stored as subscript ត.
        if options.fold_coeng_da {
            for c in &mut self.coengs {
                if *c == DA {
                    *c = TA;
                }
            }
        }

        // Lookalike: េ + េ looks like ែ.
        let mut k = 0;
        while k + 1 < self.pre.len() {
            if self.pre[k] == E && self.pre[k + 1] == E {
                self.pre[k] = AE;
                self.pre.remove(k + 1);
            }
            k += 1;
        }

        // Lookalikes: េ + ី looks like ើ, and េ + ា looks like ោ.
        if self.below.len() <= 1 {
            if let Some(last) = self.pre.last_mut() {
                if *last == E && self.above.first() == Some(&II) {
                    *last = OE;
                    self.above.remove(0);
                } else if *last == E && self.above.is_empty() && self.post.first() == Some(&AA) {
                    *last = OO;
                    self.post.remove(0);
                }
            }
        }

        // Lookalike: a ុ under an above vowel is really a shifter that the font
        // draws low. Store the shifter.
        if self.shifters.is_empty() && self.below == [U] && self.coengs.len() <= 2 {
            let strong = is_strong_cluster(self.base, &self.coengs);
            if self.u_is_a_downshifted_shifter(strong) {
                self.below.clear();
                self.shifters.push(if strong { TRIISAP } else { MUUSIKATOAN });
            }
        }
    }

    /// True when the lone ុ of this syllable is followed by something that
    /// pushes a shifter down: an above vowel, ាំ, ៝, or (weak clusters only) ័.
    fn u_is_a_downshifted_shifter(&self, strong: bool) -> bool {
        if self.pre == [OE] && self.above.is_empty() {
            return true;
        }
        let pre_ok = self.pre.len() <= 1
            && self.pre.iter().all(|v| ('\u{17C1}'..='\u{17C5}').contains(v));
        if !pre_ok {
            return false;
        }
        if !self.above.is_empty() {
            return true;
        }
        if self.post == [AA] {
            return self.signs.first() == Some(&NIKAHIT);
        }
        if self.post.is_empty() {
            return match self.signs.first() {
                Some(&ATTHACAN) => true,
                Some(&SAMYOK) => !strong,
                _ => false,
            };
        }
        false
    }

    /// Writes the syllable in its stored order.
    fn write(&self, out: &mut String) {
        out.push(self.base);
        out.extend(&self.robats);
        for &c in &self.coengs {
            out.push(COENG);
            out.push(c);
        }
        out.extend(&self.shifters);
        for _ in 0..self.zwnj {
            out.push(ZWNJ);
        }
        out.extend(&self.pre);
        out.extend(&self.below);
        out.extend(&self.above);
        out.extend(&self.post);
        out.extend(&self.signs);
        out.extend(&self.finals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The full list of cases lives in tests/data/cases.tsv. These few are here
    // so each rule has a named test next to the code that implements it.

    #[test]
    fn subscripts_go_before_the_vowel() {
        // ខ្ញុំ typed with the subscript last.
        assert_eq!(
            normalize("\u{1781}\u{17BB}\u{17C6}\u{17D2}\u{1789}"),
            "\u{1781}\u{17D2}\u{1789}\u{17BB}\u{17C6}"
        );
    }

    #[test]
    fn coeng_ro_goes_last() {
        assert_eq!(
            normalize("\u{179F}\u{17D2}\u{179A}\u{17D2}\u{178F}\u{17B8}"),
            "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"
        );
    }

    #[test]
    fn coeng_da_becomes_coeng_ta_unless_turned_off() {
        let typed = "\u{179F}\u{17D2}\u{178A}\u{17B6}";
        assert_eq!(normalize(typed), "\u{179F}\u{17D2}\u{178F}\u{17B6}");
        let keep = Options { fold_coeng_da: false };
        assert_eq!(normalize_with(typed, keep), typed);
    }

    #[test]
    fn shifter_goes_after_subscripts() {
        // ម៉្យាង
        assert_eq!(
            normalize("\u{1798}\u{17C9}\u{17D2}\u{1799}\u{17B6}\u{1784}"),
            "\u{1798}\u{17D2}\u{1799}\u{17C9}\u{17B6}\u{1784}"
        );
    }

    #[test]
    fn robat_vowel_sign_final_order() {
        assert_eq!(normalize("\u{1780}\u{17B6}\u{17CC}"), "\u{1780}\u{17CC}\u{17B6}");
        assert_eq!(normalize("\u{1780}\u{17C6}\u{17BB}"), "\u{1780}\u{17BB}\u{17C6}");
        assert_eq!(normalize("\u{1780}\u{17C7}\u{17C6}"), "\u{1780}\u{17C6}\u{17C7}");
    }

    #[test]
    fn lookalike_vowels_are_merged() {
        assert_eq!(normalize("\u{1780}\u{17C1}\u{17B6}"), "\u{1780}\u{17C4}");
        assert_eq!(normalize("\u{1780}\u{17C1}\u{17B8}"), "\u{1780}\u{17BE}");
        assert_eq!(normalize("\u{1780}\u{17C1}\u{17C1}"), "\u{1780}\u{17C2}");
    }

    #[test]
    fn u_under_an_above_vowel_becomes_the_right_shifter() {
        // ស៊ី: strong consonant, triisap.
        assert_eq!(normalize("\u{179F}\u{17BB}\u{17B8}"), "\u{179F}\u{17CA}\u{17B8}");
        // ម៉ឺ: weak consonant, muusikatoan.
        assert_eq!(normalize("\u{1798}\u{17BB}\u{17BA}"), "\u{1798}\u{17C9}\u{17BA}");
        // ប៉ិ: ba always takes muusikatoan.
        assert_eq!(normalize("\u{1794}\u{17BB}\u{17B7}"), "\u{1794}\u{17C9}\u{17B7}");
        // ញ៉ាំ
        assert_eq!(
            normalize("\u{1789}\u{17BB}\u{17B6}\u{17C6}"),
            "\u{1789}\u{17C9}\u{17B6}\u{17C6}"
        );
    }

    #[test]
    fn a_real_u_is_left_alone() {
        assert_eq!(normalize("\u{1780}\u{17BB}"), "\u{1780}\u{17BB}");
        assert_eq!(normalize("\u{1780}\u{17BB}\u{17C6}"), "\u{1780}\u{17BB}\u{17C6}");
        // ុ + ា without nikahit is two vowels, not a shifter.
        assert_eq!(normalize("\u{1780}\u{17BB}\u{17B6}"), "\u{1780}\u{17BB}\u{17B6}");
    }

    #[test]
    fn deprecated_letters_are_replaced() {
        assert_eq!(normalize("\u{17A3}"), "\u{17A2}");
        assert_eq!(normalize("\u{17A4}"), "\u{17A2}\u{17B6}");
    }

    #[test]
    fn text_that_is_not_a_syllable_is_untouched() {
        for text in ["", "hello", "\u{17B6}\u{1780}", "\u{17D2}\u{1780}\u{17B6}", "\u{1780}\u{17D2}\u{17B6}"] {
            assert_eq!(normalize(text), text);
        }
    }
}
