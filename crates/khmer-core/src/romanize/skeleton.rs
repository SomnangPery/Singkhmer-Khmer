const SAME_SOUND: &[(&str, &str)] = &[("kh", "k"), ("ch", "j"), ("ou", "o"), ("u", "o"), ("ia", "ea")];

pub(crate) fn skeleton(spelling: &str) -> String {
    let mut reduced = fold_doubled_letters(spelling);
    for (from, to) in SAME_SOUND {
        reduced = reduced.replace(from, to);
    }
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
        assert!(same("khmer", "kmer"));
        assert!(same("chong", "jong"));
        assert!(same("mouy", "moy"));
        assert!(same("mouy", "muy"));
        assert!(same("kon", "kun"));
        assert!(same("rean", "rian"));
        assert!(same("bong", "bongg"));
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
