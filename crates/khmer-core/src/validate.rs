use std::fmt;

use crate::chars::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Invalid {
    pub offset: usize,
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

pub fn validate(text: &str) -> Result<(), Invalid> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match class_of(chars[i]) {
            Class::Base => i = syllable_end(&chars, i),
            Class::Other => i += 1,
            _ => {
                let offset: usize = chars[..i].iter().map(|c| c.len_utf8()).sum();
                return Err(Invalid { offset, ch: chars[i] });
            }
        }
    }
    Ok(())
}

pub fn is_valid(text: &str) -> bool {
    validate(text).is_ok()
}

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
        assert_eq!(
            validate("\u{1780}\u{17B6}\u{17B6}"),
            Err(Invalid {
                offset: 6,
                ch: '\u{17B6}'
            })
        );
    }

    #[test]
    fn wrong_orders_are_invalid() {
        assert!(!is_valid("\u{179F}\u{17B8}\u{17D2}\u{179A}"));
        assert!(!is_valid("\u{179F}\u{17D2}\u{179A}\u{17D2}\u{178F}"));
        assert!(!is_valid("\u{1798}\u{17C9}\u{17D2}\u{1799}"));
        assert!(!is_valid("\u{1780}\u{17C6}\u{17BB}"));
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
