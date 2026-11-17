//! The validator: is this text stored the one correct way?
//!
//! It checks the Modern Khmer syllable structure of UTN #61:
//!
//! ```text
//! Syllable = Base Robat? Coengs? Shifter? Vowel? Signs? Final?
//! Coengs   = (Coeng NonRo)? Coeng Base        one or two subscripts, រ last
//! Shifter  = [៉ ៊], plus ZWNJ only where the shifter would otherwise be drawn as ុ
//! Signs    = one or two signs; ័ and ៝ may not come straight after ុ
//! ```
//!
//! Characters that are not part of a syllable (digits, punctuation, Latin
//! letters, spaces) are always fine on their own.

use std::fmt;

use crate::chars::*;

/// Where and why [`validate`] rejected a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invalid {
    /// Byte offset of the offending character in the text.
    pub offset: usize,
    /// The character that does not fit where it is.
    pub ch: char,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "U+{:04X} at byte {} does not fit the Khmer syllable structure",
            self.ch as u32, self.offset
        )
    }
}

impl std::error::Error for Invalid {}

/// Checks that every Khmer syllable in `text` is in the correct stored order.
/// On failure, reports the first character that does not fit.
pub fn validate(text: &str) -> Result<(), Invalid> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match class_of(chars[i]) {
            Class::Base => i = syllable_end(&chars, i),
            Class::Other => i += 1,
            // A vowel, sign, shifter, robat or coeng that no syllable claimed.
            _ => {
                let offset: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();
                return Err(Invalid { offset, ch: chars[i] });
            }
        }
    }
    Ok(())
}

/// True if [`validate`] accepts `text`.
pub fn is_valid(text: &str) -> bool {
    validate(text).is_ok()
}

/// Matches the longest well-formed syllable starting at the base `chars[start]`
/// and returns the index just past it.
fn syllable_end(chars: &[char], start: usize) -> usize {
    let at = |i: usize| chars.get(i).copied();
    let base = chars[start];
    let mut j = start + 1;

    if at(j) == Some(ROBAT) {
        j += 1;
    }

    let mut coengs: Vec<char> = Vec::new();
    while at(j) == Some(COENG) {
        let Some(sub) = at(j + 1).filter(|&c| is_base(c)) else {
            break;
        };
        // At most two subscripts, and nothing may follow subscript រ.
        if coengs.len() == 2 || coengs.first() == Some(&RO) {
            break;
        }
        coengs.push(sub);
        j += 2;
    }

    if let Some(shifter) = at(j).filter(|&c| class_of(c) == Class::Shifter) {
        j += 1;
        if at(j) == Some(ZWNJ) {
            let strong = is_strong_cluster(base, &coengs);
            if zwnj_allowed(strong, shifter, &chars[j + 1..]) {
                j += 1;
            }
        }
    }

    let vowel = at(j).filter(|&c| class_of(c).is_vowel());
    if vowel.is_some() {
        j += 1;
    }

    if let Some(sign) = at(j).filter(|&c| class_of(c) == Class::Sign) {
        // ុ followed by ័ or ៝ would look like a shifter drawn low, so that
        // spelling is reserved for the shifter.
        let reserved = vowel == Some(U) && (sign == SAMYOK || sign == ATTHACAN);
        if !reserved {
            j += 1;
            if at(j).map_or(false, |c| class_of(c) == Class::Sign) {
                j += 1;
            }
        }
    }

    if at(j).map_or(false, |c| class_of(c) == Class::Final) {
        j += 1;
    }
    j
}

/// A ZWNJ after a shifter is allowed only where it changes something: where,
/// without it, the font would draw the shifter as ុ. `rest` is the text after
/// the ZWNJ.
fn zwnj_allowed(strong_cluster: bool, shifter: char, rest: &[char]) -> bool {
    let first = rest.first().copied();
    let second = rest.get(1).copied();

    let above_vowel = match first {
        Some(c) if is_downshift_vowel(c) || c == ATTHACAN => true,
        Some(AA) => second == Some(NIKAHIT),
        _ => false,
    };
    let samyok = match first {
        Some(SAMYOK) => true,
        Some('\u{17C1}'..='\u{17C3}') => second == Some(SAMYOK),
        _ => false,
    };

    if shifter == TRIISAP {
        strong_cluster && above_vowel
    } else {
        !strong_cluster && (above_vowel || samyok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_words_are_valid() {
        // ស្ត្រី, ខ្ញុំ, ម៉្យាង (shifter after the subscript), ឱ្យ
        for word in [
            "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}",
            "\u{1781}\u{17D2}\u{1789}\u{17BB}\u{17C6}",
            "\u{1798}\u{17D2}\u{1799}\u{17C9}\u{17B6}\u{1784}",
            "\u{17B1}\u{17D2}\u{1799}",
        ] {
            assert_eq!(validate(word), Ok(()));
        }
    }

    #[test]
    fn reports_the_first_character_that_does_not_fit() {
        // កាា: the second ា is at byte 6 (each Khmer character is 3 bytes).
        assert_eq!(
            validate("\u{1780}\u{17B6}\u{17B6}"),
            Err(Invalid { offset: 6, ch: '\u{17B6}' })
        );
    }

    #[test]
    fn wrong_orders_are_invalid() {
        // subscript after the vowel
        assert!(!is_valid("\u{179F}\u{17B8}\u{17D2}\u{179A}"));
        // coeng ro before another subscript
        assert!(!is_valid("\u{179F}\u{17D2}\u{179A}\u{17D2}\u{178F}"));
        // shifter before the subscript
        assert!(!is_valid("\u{1798}\u{17C9}\u{17D2}\u{1799}"));
        // sign before the vowel
        assert!(!is_valid("\u{1780}\u{17C6}\u{17BB}"));
        // robat after the vowel
        assert!(!is_valid("\u{1780}\u{17B6}\u{17CC}"));
    }

    #[test]
    fn marks_need_a_base() {
        assert!(!is_valid("\u{17B6}"));
        assert!(!is_valid("\u{17D2}\u{1780}"));
        assert!(!is_valid("\u{1780}\u{17D2}"));
    }

    #[test]
    fn zwnj_only_where_the_shifter_would_drop() {
        assert!(is_valid("\u{179F}\u{17CA}\u{200C}\u{17B8}"));
        assert!(!is_valid("\u{1794}\u{17CA}\u{200C}\u{17B8}"));
        assert!(is_valid("\u{1794}\u{17C9}\u{200C}\u{17B7}"));
        assert!(!is_valid("\u{179F}\u{17CA}\u{200C}\u{17B6}"));
        assert!(is_valid("\u{179F}\u{17CA}\u{200C}\u{17B6}\u{17C6}"));
    }

    #[test]
    fn other_characters_are_fine_on_their_own() {
        assert!(is_valid(""));
        assert!(is_valid("ok 123 \u{17E1}\u{17D4}"));
    }
}
