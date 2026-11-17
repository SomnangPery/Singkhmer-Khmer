use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::ops::Bound;

use super::skeleton::skeleton;
use crate::normalize::normalize;
use crate::validate::is_valid;

const SPELLING_DECAY: f32 = 0.6;
const MAX_CANDIDATES: usize = 30;

const VARIANT: f32 = -2.5;
const PREFIX: f32 = -2.0;
const PREFIX_PER_LETTER: f32 = -1.0;
const LATIN_ENGLISH: f32 = -4.0;
const LATIN_ENGLISH_WITH_KHMER: f32 = -12.0;
const LATIN_OTHER: f32 = -16.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataError {
    pub file: &'static str,
    pub line: usize,
    pub message: String,
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} line {}: {}", self.file, self.line, self.message)
    }
}

impl std::error::Error for DataError {}

struct Listed {
    word: usize,
    score: f32,
}

pub(crate) struct Candidate {
    pub text: String,
    pub latin: bool,
    pub score: f32,
    /// The score is final: the language model is not asked about this word.
    pub fixed: bool,
}

struct Line {
    word: String,
    count: f32,
    spellings: Vec<String>,
}

pub struct Lexicon {
    words: Vec<(String, f32)>,
    shares: Vec<f32>,
    index: HashMap<String, usize>,
    spellings: BTreeMap<String, Vec<Listed>>,
    skeletons: HashMap<String, Vec<Listed>>,
    checked_spellings: HashSet<String>,
    english: HashSet<String>,
}

impl Lexicon {
    pub fn from_tsv(words: &str, imported: &str, english: &str) -> Result<Lexicon, DataError> {
        let mut lines: Vec<Line> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut checked_spellings = HashSet::new();

        for (line, text) in data_lines(words) {
            let error = |message| DataError {
                file: "words",
                line,
                message,
            };
            let parsed = parse_line(text).map_err(error)?;
            if index.contains_key(&parsed.word) {
                return Err(error(format!("\"{}\" is listed twice", parsed.word)));
            }
            checked_spellings.extend(parsed.spellings.iter().cloned());
            index.insert(parsed.word.clone(), lines.len());
            lines.push(parsed);
        }
        for (line, text) in data_lines(imported) {
            let parsed = parse_line(text).map_err(|message| DataError {
                file: "words-imported",
                line,
                message,
            })?;
            match index.get(&parsed.word) {
                Some(&known) => {
                    for spelling in parsed.spellings {
                        if !lines[known].spellings.contains(&spelling) {
                            lines[known].spellings.push(spelling);
                        }
                    }
                }
                None => {
                    index.insert(parsed.word.clone(), lines.len());
                    lines.push(parsed);
                }
            }
        }

        let total: f32 = lines.iter().map(|line| line.count).sum();
        let mut lexicon = Lexicon {
            words: Vec::with_capacity(lines.len()),
            shares: Vec::with_capacity(lines.len()),
            index,
            spellings: BTreeMap::new(),
            skeletons: HashMap::new(),
            checked_spellings,
            english: HashSet::new(),
        };
        for (id, line) in lines.into_iter().enumerate() {
            let weights: Vec<f32> = (0..line.spellings.len())
                .map(|rank| SPELLING_DECAY.powi(rank as i32))
                .collect();
            let weight_total: f32 = weights.iter().sum();
            for (spelling, weight) in line.spellings.iter().zip(weights) {
                let score = (weight / weight_total).ln();
                lexicon
                    .spellings
                    .entry(spelling.clone())
                    .or_default()
                    .push(Listed { word: id, score });
                let same_skeleton = lexicon.skeletons.entry(skeleton(spelling)).or_default();
                match same_skeleton.iter_mut().find(|listed| listed.word == id) {
                    Some(listed) => listed.score = listed.score.max(score),
                    None => same_skeleton.push(Listed { word: id, score }),
                }
            }
            lexicon.shares.push((line.count / total).ln());
            lexicon.words.push((line.word, line.count));
        }
        for (_, text) in data_lines(english) {
            lexicon.english.insert(text.trim().to_ascii_lowercase());
        }
        Ok(lexicon)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn contains(&self, word: &str) -> bool {
        self.index.contains_key(word)
    }

    pub fn words(&self) -> impl Iterator<Item = (&str, f32)> {
        self.words.iter().map(|(word, count)| (word.as_str(), *count))
    }

    pub(crate) fn candidates(&self, token: &str) -> Vec<Candidate> {
        if token.is_empty() || !token.chars().all(|c| c.is_ascii_alphabetic()) {
            return vec![Candidate {
                text: token.to_string(),
                latin: true,
                score: 0.0,
                fixed: true,
            }];
        }
        let typed = token.to_ascii_lowercase();
        let mut found: BTreeMap<usize, f32> = BTreeMap::new();
        let mut keep_best = |word: usize, score: f32| {
            let best = found.entry(word).or_insert(f32::NEG_INFINITY);
            *best = best.max(score);
        };

        let exact = self.spellings.get(&typed);
        for listed in exact.into_iter().flatten() {
            keep_best(listed.word, listed.score);
        }
        for listed in self.skeletons.get(&skeleton(&typed)).into_iter().flatten() {
            keep_best(listed.word, listed.score + VARIANT);
        }
        let longer = (Bound::Excluded(typed.as_str()), Bound::Unbounded);
        for (spelling, words) in self.spellings.range::<str, _>(longer) {
            if !spelling.starts_with(&typed) {
                break;
            }
            let missing = (spelling.len() - typed.len()) as f32;
            for listed in words {
                keep_best(listed.word, listed.score + PREFIX + missing * PREFIX_PER_LETTER);
            }
        }

        let mut found: Vec<(usize, f32)> = found.into_iter().collect();
        if found.len() > MAX_CANDIDATES {
            let worth = |&(word, score): &(usize, f32)| score + self.shares[word];
            found.sort_by(|a, b| worth(b).total_cmp(&worth(a)).then(a.0.cmp(&b.0)));
            found.truncate(MAX_CANDIDATES);
        }
        let mut candidates: Vec<Candidate> = found
            .into_iter()
            .map(|(word, score)| Candidate {
                text: self.words[word].0.clone(),
                latin: false,
                score,
                fixed: false,
            })
            .collect();

        let checked = self.checked_spellings.contains(&typed);
        let score = match (self.english.contains(&typed), checked) {
            (true, false) => LATIN_ENGLISH,
            (true, true) => LATIN_ENGLISH_WITH_KHMER,
            (false, _) => LATIN_OTHER,
        };
        candidates.push(Candidate {
            text: token.to_string(),
            latin: true,
            score,
            fixed: true,
        });
        candidates
    }
}

fn parse_line(line: &str) -> Result<Line, String> {
    let mut columns = line.split('\t');
    let word = normalize(columns.next().unwrap_or("").trim());
    let count = columns.next().unwrap_or("").trim();
    let spellings = columns.next().unwrap_or("").trim();

    if word.is_empty() || !word.chars().all(is_khmer) {
        return Err(format!("\"{word}\" is not a Khmer word"));
    }
    if !is_valid(&word) {
        return Err(format!("\"{word}\" is not a valid Khmer syllable sequence"));
    }
    let count: f32 = match count.parse() {
        Ok(count) if count > 0.0 => count,
        _ => return Err(format!("\"{count}\" should be a number above zero")),
    };
    let spellings: Vec<String> = spellings.split(',').map(|s| s.trim().to_string()).collect();
    let mut seen = HashSet::new();
    for spelling in &spellings {
        if spelling.is_empty() || !spelling.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(format!("spelling \"{spelling}\" should be letters a-z only"));
        }
        if !seen.insert(spelling.as_str()) {
            return Err(format!("spelling \"{spelling}\" is listed twice"));
        }
    }
    Ok(Line { word, count, spellings })
}

pub(crate) fn data_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'))
}

fn is_khmer(c: char) -> bool {
    ('\u{1780}'..='\u{17FF}').contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORDS: &str = "\
# comment
ខ្ញុំ\t9000\tnh, knhom\tI
ញ៉ាំ\t2500\tnham, nh\teat
សាលា\t2500\tsala\tschool
ចង់\t6000\tjg, jong, chong\twant
";

    const ENGLISH: &str = "ok\nfacebook\nsala\n";

    fn lexicon() -> Lexicon {
        Lexicon::from_tsv(WORDS, "", ENGLISH).unwrap()
    }

    fn khmer(lexicon: &Lexicon, token: &str) -> Vec<String> {
        let mut candidates = lexicon.candidates(token);
        candidates.retain(|c| !c.latin);
        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates.into_iter().map(|c| c.text).collect()
    }

    #[test]
    fn exact_spellings_find_every_word_that_lists_them() {
        assert_eq!(khmer(&lexicon(), "nh"), ["ខ្ញុំ", "ញ៉ាំ"]);
    }

    #[test]
    fn capital_letters_match_too() {
        assert_eq!(khmer(&lexicon(), "NH"), ["ខ្ញុំ", "ញ៉ាំ"]);
    }

    #[test]
    fn variant_rules_find_unlisted_spellings() {
        assert_eq!(khmer(&lexicon(), "khnhom"), ["ខ្ញុំ"]);
        assert_eq!(khmer(&lexicon(), "salaa"), ["សាលា"]);
    }

    #[test]
    fn a_listed_spelling_beats_a_variant() {
        let lexicon = lexicon();
        let listed = lexicon.candidates("chong").remove(0).score;
        let variant = lexicon.candidates("jongg").remove(0).score;
        assert!(listed > variant);
    }

    #[test]
    fn the_start_of_a_spelling_matches() {
        assert_eq!(khmer(&lexicon(), "sal"), ["សាលា"]);
    }

    #[test]
    fn the_token_itself_is_always_a_candidate() {
        let lexicon = lexicon();
        for token in ["nh", "xyz", "Facebook", "2026", "ok!"] {
            let candidates = lexicon.candidates(token);
            let latin: Vec<_> = candidates.iter().filter(|c| c.latin).collect();
            assert_eq!(latin.len(), 1, "{token}");
            assert_eq!(latin[0].text, token);
        }
    }

    #[test]
    fn english_words_are_kept_unless_khmer_is_spelled_exactly_so() {
        let lexicon = lexicon();
        let latin_score = |token: &str| {
            let candidates = lexicon.candidates(token);
            candidates.iter().find(|c| c.latin).unwrap().score
        };
        assert_eq!(latin_score("facebook"), LATIN_ENGLISH);
        assert_eq!(latin_score("sala"), LATIN_ENGLISH_WITH_KHMER);
        assert_eq!(latin_score("xyz"), LATIN_OTHER);
    }

    #[test]
    fn words_are_normalized_when_read() {
        let wrong_order = "\u{179F}\u{17B8}\u{17D2}\u{179A}\u{17D2}\u{178F}\t10\tsatrey\n";
        let lexicon = Lexicon::from_tsv(wrong_order, "", "").unwrap();
        assert!(lexicon.contains("\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"));
    }

    #[test]
    fn imported_words_add_to_the_checked_list() {
        let imported = "\
ខ្ញុំ\t20\tnhom, nh, knhom
អូខេ\t20\tok
ហែង\t20\theng, hg
";
        let lexicon = Lexicon::from_tsv(WORDS, imported, ENGLISH).unwrap();
        assert_eq!(lexicon.len(), 6);
        assert_eq!(khmer(&lexicon, "hg"), ["ហែង"]);
        let count = |word: &str| lexicon.words().find(|(w, _)| *w == word).unwrap().1;
        assert_eq!(count("ខ្ញុំ"), 9000.0);
        assert_eq!(khmer(&lexicon, "nhom"), ["ខ្ញុំ"]);
        let score = |token: &str| lexicon.candidates(token).remove(0).score;
        assert!(score("knhom") > score("nhom"));
    }

    #[test]
    fn an_imported_spelling_does_not_take_over_an_english_word() {
        let imported = "អូខេ\t20\tok\n";
        let lexicon = Lexicon::from_tsv(WORDS, imported, ENGLISH).unwrap();
        let candidates = lexicon.candidates("ok");
        let latin = candidates.iter().find(|c| c.latin).unwrap();
        assert_eq!(latin.score, LATIN_ENGLISH);
        assert!(candidates.iter().any(|c| c.text == "អូខេ"));
    }

    #[test]
    fn a_short_token_keeps_only_the_best_candidates() {
        let mut words = String::new();
        for number in 0..100u32 {
            let consonant = char::from_u32(0x1780 + number % 30).unwrap();
            let vowel = char::from_u32(0x17B6 + number / 30).unwrap();
            let tail: String = number.to_string().bytes().map(|b| (b - b'0' + b'a') as char).collect();
            words.push_str(&format!("{consonant}{vowel}\t{}\tka{tail}\n", number + 1));
        }
        let lexicon = Lexicon::from_tsv(&words, "", "").unwrap();
        assert_eq!(lexicon.len(), 100);
        let khmer = lexicon.candidates("ka").iter().filter(|c| !c.latin).count();
        assert_eq!(khmer, MAX_CANDIDATES);
    }

    #[test]
    fn bad_lines_are_reported_with_their_line_number() {
        let error = |text: &str| Lexicon::from_tsv(text, "", "").err().unwrap();
        assert_eq!(
            Lexicon::from_tsv(WORDS, "x\t1\tx\n", "").err().unwrap().file,
            "words-imported"
        );
        assert_eq!(error("# ok\nhello\t10\thello\n").line, 2);
        assert!(error("ខ្ញុំ\tmany\tnh\n").message.contains("number"));
        assert!(error("ខ្ញុំ\t10\tNh\n").message.contains("a-z"));
        assert!(error("ខ្ញុំ\t10\tnh, nh\n").message.contains("twice"));
        assert!(error("ខ្ញុំ\t10\tnh\nខ្ញុំ\t10\tknhom\n").message.contains("twice"));
        assert!(error("\u{17B6}\t10\ta\n").message.contains("valid"));
    }
}
