//! Khmer character classes.
//!
//! This is the table the rest of the engine is built on. Each Khmer character
//! belongs to exactly one [`Class`], and a syllable is stored with its classes in
//! this order:
//!
//! ```text
//! Base  Robat?  (Coeng Base){0,2}  Shifter?  Vowel?  Sign{0,2}  Final?
//! ```

/// ៌ robat.
pub const ROBAT: char = '\u{17CC}';
/// ្ the coeng sign: it turns the consonant after it into a subscript.
pub const COENG: char = '\u{17D2}';
/// Zero width non-joiner. After a shifter it stops the shifter being drawn as ុ.
pub const ZWNJ: char = '\u{200C}';

/// រ ro. Its subscript (្រ) is always stored after any other subscript.
pub const RO: char = '\u{179A}';
/// ប ba. It behaves differently from every other consonant under a shifter.
pub const BA: char = '\u{1794}';
/// ដ da. Its subscript looks identical to the subscript of ត ta.
pub const DA: char = '\u{178A}';
/// ត ta.
pub const TA: char = '\u{178F}';

/// ៉ muusikatoan.
pub const MUUSIKATOAN: char = '\u{17C9}';
/// ៊ triisap.
pub const TRIISAP: char = '\u{17CA}';

/// ា vowel aa.
pub const AA: char = '\u{17B6}';
/// ី vowel ii.
pub const II: char = '\u{17B8}';
/// ុ vowel u.
pub const U: char = '\u{17BB}';
/// ើ vowel oe.
pub const OE: char = '\u{17BE}';
/// េ vowel e.
pub const E: char = '\u{17C1}';
/// ែ vowel ae.
pub const AE: char = '\u{17C2}';
/// ោ vowel oo.
pub const OO: char = '\u{17C4}';

/// ំ nikahit.
pub const NIKAHIT: char = '\u{17C6}';
/// ័ samyok sannya.
pub const SAMYOK: char = '\u{17D0}';
/// ៝ atthacan.
pub const ATTHACAN: char = '\u{17DD}';

/// The role a character plays inside a syllable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// A consonant or independent vowel: the start of a syllable.
    Base,
    /// ៌
    Robat,
    /// ្
    Coeng,
    /// ៉ or ៊
    Shifter,
    /// A vowel with a part drawn before the consonant (ើ ឿ ៀ េ ែ ៃ ោ ៅ).
    VowelPre,
    /// A vowel drawn below (ុ ូ ួ).
    VowelBelow,
    /// A vowel drawn above (ិ ី ឹ ឺ).
    VowelAbove,
    /// The vowel drawn after (ា).
    VowelPost,
    /// A sign drawn above the syllable (ំ ់ ៍ ៎ ៏ ័ ៑ ៝).
    Sign,
    /// A sign drawn after the syllable (ះ ៈ).
    Final,
    /// Anything that is not part of a syllable: digits, punctuation, Latin
    /// letters, spaces, and the deprecated Khmer characters.
    Other,
}

impl Class {
    /// True for the four vowel classes.
    pub fn is_vowel(self) -> bool {
        matches!(
            self,
            Class::VowelPre | Class::VowelBelow | Class::VowelAbove | Class::VowelPost
        )
    }
}

/// The class of one character.
pub fn class_of(c: char) -> Class {
    match c {
        '\u{1780}'..='\u{17A2}' | '\u{17A5}'..='\u{17B3}' => Class::Base,
        '\u{17B6}' => Class::VowelPost,
        '\u{17B7}'..='\u{17BA}' => Class::VowelAbove,
        '\u{17BB}'..='\u{17BD}' => Class::VowelBelow,
        '\u{17BE}'..='\u{17C5}' => Class::VowelPre,
        '\u{17C6}' | '\u{17CB}' | '\u{17CD}'..='\u{17D1}' | '\u{17DD}' => Class::Sign,
        '\u{17C7}' | '\u{17C8}' => Class::Final,
        '\u{17C9}' | '\u{17CA}' => Class::Shifter,
        '\u{17CC}' => Class::Robat,
        '\u{17D2}' => Class::Coeng,
        _ => Class::Other,
    }
}

/// True for consonants and independent vowels.
pub fn is_base(c: char) -> bool {
    class_of(c) == Class::Base
}

/// True for the "strong" consonants: the ones that are first series, or that
/// outrank a sonorant when they share a cluster. (កខគឃ ចឆជឈ ដឋឌឍ តថទធ ផពភ ឞសហ អ)
pub fn is_strong_consonant(c: char) -> bool {
    matches!(
        c,
        '\u{1780}'..='\u{1783}'
            | '\u{1785}'..='\u{1788}'
            | '\u{178A}'..='\u{178D}'
            | '\u{178F}'..='\u{1792}'
            | '\u{1795}'..='\u{1797}'
            | '\u{179E}'..='\u{17A0}'
            | '\u{17A2}'
    )
}

/// Decides which shifter a cluster (base plus subscripts) pairs with.
///
/// A cluster is strong when it contains a strong consonant and no ប. On a
/// strong cluster it is ៊ (triisap) that is drawn as ុ under an above vowel; on
/// every other cluster it is ៉ (muusikatoan).
pub fn is_strong_cluster(base: char, coengs: &[char]) -> bool {
    let mut members = std::iter::once(base).chain(coengs.iter().copied());
    let all_plain_consonants = members
        .clone()
        .all(|c| ('\u{1780}'..='\u{17A2}').contains(&c) && c != BA);
    all_plain_consonants && members.any(is_strong_consonant)
}

/// True for the single vowels that push a shifter down (ិ ី ឹ ឺ ើ).
pub fn is_downshift_vowel(c: char) -> bool {
    matches!(c, '\u{17B7}'..='\u{17BA}' | '\u{17BE}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deprecated_letters_are_not_bases() {
        assert_eq!(class_of('\u{17A3}'), Class::Other);
        assert_eq!(class_of('\u{17A4}'), Class::Other);
        assert_eq!(class_of('\u{17A2}'), Class::Base);
        assert_eq!(class_of('\u{17A5}'), Class::Base);
    }

    #[test]
    fn strong_and_weak_clusters() {
        // ស is strong.
        assert!(is_strong_cluster('\u{179F}', &[]));
        // ម is a sonorant: weak.
        assert!(!is_strong_cluster('\u{1798}', &[]));
        // ម្ស: one strong member makes the whole cluster strong.
        assert!(is_strong_cluster('\u{1798}', &['\u{179F}']));
        // ប is never part of a strong cluster.
        assert!(!is_strong_cluster(BA, &[]));
        assert!(!is_strong_cluster('\u{179F}', &[BA]));
        // Independent vowels count as weak.
        assert!(!is_strong_cluster('\u{17B1}', &[]));
    }
}
