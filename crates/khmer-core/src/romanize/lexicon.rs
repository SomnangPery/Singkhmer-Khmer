use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

use super::skeleton::skeleton;
use super::strings::Strings;
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

/// One spelling (or variant skeleton) of one word.
#[derive(Clone, Copy)]
struct Listed {
    /// The spelling's number in `Lexicon::spellings` or `Lexicon::skeleton_texts`.
    text: u32,
    word: u32,
    score: f32,
}

pub(crate) struct Candidate {
    pub text: String,
    pub latin: bool,
    pub score: f32,
    /// The score is final: the language model is not asked about this word.
    pub fixed: bool,
}

struct Line<'a> {
    count: f32,
    spellings: Vec<&'a str>,
}

/// The words and their spellings, packed tightly (see `strings.rs`): sorted
/// lists searched by halving instead of maps of separate strings.
pub struct Lexicon {
    /// Word number to Khmer text.
    words: Strings,
    counts: Vec<f32>,
    shares: Vec<f32>,
    /// Word numbers in the order of their text, to find a word.
    by_text: Vec<u32>,
    /// Each spelling once, in sorted order.
    spellings: Strings,
    /// Sorted by spelling.
    spelled: Vec<Listed>,
    skeleton_texts: Strings,
    /// Sorted by skeleton, then word; one entry per skeleton and word.
    skeletons: Vec<Listed>,
    checked_spellings: HashSet<String>,
    english: HashSet<String>,
}

impl Lexicon {
    pub fn from_tsv(words: &str, imported: &str, english: &str) -> Result<Lexicon, DataError> {
        let most = words.lines().count() + imported.lines().count();
        let mut texts = Strings::default();
        let mut lines: Vec<Line> = Vec::with_capacity(most);
        let mut index = WordIndex::new(most);
        let mut checked_spellings = HashSet::new();

        for (line, text) in data_lines(words) {
            let error = |message| DataError {
                file: "words",
                line,
                message,
            };
            let (word, parsed) = parse_line(text).map_err(error)?;
            if index.find_or_add(&mut texts, &word).is_some() {
                return Err(error(format!("\"{word}\" is listed twice")));
            }
            checked_spellings.extend(parsed.spellings.iter().map(|s| s.to_string()));
            lines.push(parsed);
        }
        for (line, text) in data_lines(imported) {
            let (word, parsed) = parse_line(text).map_err(|message| DataError {
                file: "words-imported",
                line,
                message,
            })?;
            match index.find_or_add(&mut texts, &word) {
                Some(known) => {
                    let known = &mut lines[known as usize].spellings;
                    for spelling in parsed.spellings {
                        if !known.contains(&spelling) {
                            known.push(spelling);
                        }
                    }
                }
                None => lines.push(parsed),
            }
        }
        drop(index);

        // Built one step at a time, each step's lists freed before the next,
        // so that reading the dictionary never needs much more memory than
        // keeping it.
        let total: f32 = lines.iter().map(|line| line.count).sum();
        let entries: usize = lines.iter().map(|line| line.spellings.len()).sum();
        let mut counts = Vec::with_capacity(lines.len());
        let mut shares = Vec::with_capacity(lines.len());
        let mut spelled: Vec<(&str, u32, f32)> = Vec::with_capacity(entries);
        for (id, line) in lines.into_iter().enumerate() {
            let weights: Vec<f32> = (0..line.spellings.len())
                .map(|rank| SPELLING_DECAY.powi(rank as i32))
                .collect();
            let weight_total: f32 = weights.iter().sum();
            for (spelling, weight) in line.spellings.iter().copied().zip(weights) {
                spelled.push((spelling, id as u32, (weight / weight_total).ln()));
            }
            shares.push((line.count / total).ln());
            counts.push(line.count);
        }

        // Each spelling once, sorted; the entries point at it.
        spelled.sort_unstable_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(&b.1)));
        let mut spellings = Strings::default();
        let listed: Vec<Listed> = spelled
            .iter()
            .enumerate()
            .map(|(at, &(spelling, word, score))| {
                if at == 0 || spelled[at - 1].0 != spelling {
                    spellings.push(spelling);
                }
                Listed { text: (spellings.len() - 1) as u32, word, score }
            })
            .collect();
        drop(spelled);
        let spelled = listed;

        // The same for the skeletons, keeping the best score of each word.
        let mut skeleton_list = Strings::default();
        let mut skeletons: Vec<Listed> = Vec::with_capacity(spelled.len());
        for (at, listed) in spelled.iter().enumerate() {
            if at == 0 || spelled[at - 1].text != listed.text {
                skeleton_list.push(&skeleton(spellings.get(listed.text)));
            }
            skeletons.push(Listed { text: (skeleton_list.len() - 1) as u32, ..*listed });
        }
        skeletons.sort_unstable_by(|a, b| {
            skeleton_list.get(a.text).cmp(skeleton_list.get(b.text)).then(a.word.cmp(&b.word))
        });
        let mut skeleton_texts = Strings::default();
        let mut kept = 0;
        let mut last: Option<&str> = None;
        for at in 0..skeletons.len() {
            let listed = skeletons[at];
            let text = skeleton_list.get(listed.text);
            if last != Some(text) {
                skeleton_texts.push(text);
                last = Some(text);
            } else if skeletons[kept - 1].word == listed.word {
                skeletons[kept - 1].score = skeletons[kept - 1].score.max(listed.score);
                continue;
            }
            skeletons[kept] = Listed { text: (skeleton_texts.len() - 1) as u32, ..listed };
            kept += 1;
        }
        skeletons.truncate(kept);
        drop(skeleton_list);

        let mut by_text: Vec<u32> = (0..texts.len() as u32).collect();
        by_text.sort_by(|&a, &b| texts.get(a).cmp(texts.get(b)));
        texts.shrink_to_fit();
        spellings.shrink_to_fit();
        skeleton_texts.shrink_to_fit();
        skeletons.shrink_to_fit();

        let mut lexicon = Lexicon {
            words: texts,
            counts,
            shares,
            by_text,
            spellings,
            spelled,
            skeleton_texts,
            skeletons,
            checked_spellings,
            english: HashSet::new(),
        };
        for (_, text) in data_lines(english) {
            lexicon.english.insert(text.trim().to_ascii_lowercase());
        }
        Ok(lexicon)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.len() == 0
    }

    pub fn contains(&self, word: &str) -> bool {
        self.by_text.binary_search_by(|&id| self.words.get(id).cmp(word)).is_ok()
    }

    pub fn words(&self) -> impl Iterator<Item = (&str, f32)> {
        self.words.iter().zip(self.counts.iter().copied())
    }

    /// The entries of `list` whose text is `key`.
    fn listed<'a>(list: &'a [Listed], texts: &Strings, key: &str) -> &'a [Listed] {
        let start = list.partition_point(|listed| texts.get(listed.text) < key);
        let end = start + list[start..].partition_point(|listed| texts.get(listed.text) == key);
        &list[start..end]
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
        let mut found: BTreeMap<u32, f32> = BTreeMap::new();
        let mut keep_best = |word: u32, score: f32| {
            let best = found.entry(word).or_insert(f32::NEG_INFINITY);
            *best = best.max(score);
        };

        for listed in Lexicon::listed(&self.spelled, &self.spellings, &typed) {
            keep_best(listed.word, listed.score);
        }
        for listed in Lexicon::listed(&self.skeletons, &self.skeleton_texts, &skeleton(&typed)) {
            keep_best(listed.word, listed.score + VARIANT);
        }
        let longer = self.spelled.partition_point(|listed| self.spellings.get(listed.text) <= typed.as_str());
        for listed in &self.spelled[longer..] {
            let spelling = self.spellings.get(listed.text);
            if !spelling.starts_with(&typed) {
                break;
            }
            let missing = (spelling.len() - typed.len()) as f32;
            keep_best(listed.word, listed.score + PREFIX + missing * PREFIX_PER_LETTER);
        }

        let mut found: Vec<(u32, f32)> = found.into_iter().collect();
        if found.len() > MAX_CANDIDATES {
            let worth = |&(word, score): &(u32, f32)| score + self.shares[word as usize];
            found.sort_by(|a, b| worth(b).total_cmp(&worth(a)).then(a.0.cmp(&b.0)));
            found.truncate(MAX_CANDIDATES);
        }
        let mut candidates: Vec<Candidate> = found
            .into_iter()
            .map(|(word, score)| Candidate {
                text: self.words.get(word).to_string(),
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

/// Word numbers by text, while the dictionary is read: a table of numbers
/// into `Strings` rather than a map holding a second copy of every word.
struct WordIndex {
    slots: Vec<u32>,
}

impl WordIndex {
    const EMPTY: u32 = u32::MAX;

    /// Room for `most` words, never more.
    fn new(most: usize) -> WordIndex {
        WordIndex { slots: vec![WordIndex::EMPTY; (most * 2).next_power_of_two().max(2)] }
    }

    /// The number of `word` if it was added before; otherwise adds it to
    /// `texts` and returns None.
    fn find_or_add(&mut self, texts: &mut Strings, word: &str) -> Option<u32> {
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        let mask = self.slots.len() - 1;
        let mut slot = hasher.finish() as usize & mask;
        loop {
            match self.slots[slot] {
                WordIndex::EMPTY => {
                    self.slots[slot] = texts.push(word);
                    return None;
                }
                id if texts.get(id) == word => return Some(id),
                _ => slot = (slot + 1) & mask,
            }
        }
    }
}

fn parse_line(line: &str) -> Result<(String, Line<'_>), String> {
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
    let spellings: Vec<&str> = spellings.split(',').map(str::trim).collect();
    let mut seen = HashSet::new();
    for spelling in &spellings {
        if spelling.is_empty() || !spelling.chars().all(|c| c.is_ascii_lowercase()) {
            return Err(format!("spelling \"{spelling}\" should be letters a-z only"));
        }
        if !seen.insert(*spelling) {
            return Err(format!("spelling \"{spelling}\" is listed twice"));
        }
    }
    Ok((word, Line { count, spellings }))
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
