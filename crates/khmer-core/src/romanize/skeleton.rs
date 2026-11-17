//! Spelling variants.
//!
//! People romanize the same Khmer sound in several ways: "khmer" or "kmer",
//! "chong" or "jong", "mouy" or "muy". Instead of listing every combination in
//! the dictionary, each spelling is reduced to a "skeleton" in which those
//! differences are gone. Two spellings with the same skeleton are variants of
//! each other.

/// Letter groups that are treated as the same sound: the left side is rewritten
/// to the right side. Rules are applied top to bottom, so a longer group must
/// come before a shorter one that it contains ("ou" before "u").
///
/// These are the rules from the design document. To add one, add a line here
/// and a case to the test below.
const SAME_SOUND: &[(&str, &str)] = &[
    ("kh", "k"),
    ("ch", "j"),
    ("ou", "o"),
    ("u", "o"),
    ("ia", "ea"),
];

/// Reduces a lowercase spelling to its skeleton.
///
/// Doubled letters are also folded into one ("bongg" and "bong" match).
pub(crate) fn skeleton(spelling: &str) -> String {
    let mut reduced = fold_doubled_letters(spelling);
    for (from, to) in SAME_SOUND {
        reduced = reduced.replace(from, to);
    }
    // A rule can create a new double ("muoy" -> "mooy"), so fold again.
    fold_doubled_letters(&reduced)
}

fn fold_doubled_letters(spelling: &str) -> String {
    let mut out = String::with_capacity(spelling.len());
    for letter in spelling.chars() {
        if !out.ends_with(letter) {
            out.push(letter);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &str, b: &str) -> bool {
        skeleton(a) == skeleton(b)
    }

    #[test]
    fn each_variant_rule() {
        assert!(same("khmer", "kmer")); // kh <-> k
        assert!(same("chong", "jong")); // ch <-> j
        assert!(same("mouy", "moy")); // ou <-> o
        assert!(same("mouy", "muy")); // ou <-> u
        assert!(same("kon", "kun")); // o <-> u
        assert!(same("rean", "rian")); // ea <-> ia
        assert!(same("bong", "bongg")); // doubled letters
        assert!(same("saat", "sat"));
    }

    #[test]
    fn rules_combine() {
        assert!(same("chhoul", "jol"));
        assert!(same("muoy", "moy"));
    }

    #[test]
    fn different_words_stay_different() {
        assert!(!same("ban", "bat"));
        assert!(!same("tv", "te"));
        assert!(!same("mean", "men"));
    }
}
