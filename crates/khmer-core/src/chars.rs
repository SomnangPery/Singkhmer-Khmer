pub const ROBAT: char = '\u{17CC}';
pub const COENG: char = '\u{17D2}';
pub const ZWNJ: char = '\u{200C}';

pub const RO: char = '\u{179A}';
pub const BA: char = '\u{1794}';
pub const DA: char = '\u{178A}';
pub const TA: char = '\u{178F}';

pub const MUUSIKATOAN: char = '\u{17C9}';
pub const TRIISAP: char = '\u{17CA}';

pub const AA: char = '\u{17B6}';
pub const II: char = '\u{17B8}';
pub const U: char = '\u{17BB}';
pub const OE: char = '\u{17BE}';
pub const E: char = '\u{17C1}';
pub const AE: char = '\u{17C2}';
pub const OO: char = '\u{17C4}';

pub const NIKAHIT: char = '\u{17C6}';
pub const SAMYOK: char = '\u{17D0}';
pub const ATTHACAN: char = '\u{17DD}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Base,
    Robat,
    Coeng,
    Shifter,
    VowelPre,
    VowelBelow,
    VowelAbove,
    VowelPost,
    Sign,
    Final,
    Other,
}

impl Class {
    pub fn is_vowel(self) -> bool {
        matches!(
            self,
            Class::VowelPre | Class::VowelBelow | Class::VowelAbove | Class::VowelPost
        )
    }
}

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

pub fn is_base(c: char) -> bool {
    class_of(c) == Class::Base
}

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

pub fn is_strong_cluster(base: char, coengs: &[char]) -> bool {
    let mut members = std::iter::once(base).chain(coengs.iter().copied());
    let all_plain_consonants = members
        .clone()
        .all(|c| ('\u{1780}'..='\u{17A2}').contains(&c) && c != BA);
    all_plain_consonants && members.any(is_strong_consonant)
}

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
        assert!(is_strong_cluster('\u{179F}', &[]));
        assert!(!is_strong_cluster('\u{1798}', &[]));
        assert!(is_strong_cluster('\u{1798}', &['\u{179F}']));
        assert!(!is_strong_cluster(BA, &[]));
        assert!(!is_strong_cluster('\u{179F}', &[BA]));
        assert!(!is_strong_cluster('\u{17B1}', &[]));
    }
}
