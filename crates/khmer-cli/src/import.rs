//! `khmer import`: turns a file of Khmer/Latin pairs from another source into
//! a words file the engine can load (`words-imported.tsv`).
//!
//! The input has one pair per line, in any of these shapes:
//!
//! ```text
//! ខ្ញុំ: knhom
//! knhom,ខ្ញុំ,any,other,columns
//! ខ្ញុំ <TAB> knhom
//! ខ្ញុំ=knhom,khnhom,nhom
//! ```
//!
//! Either side may come first, except in the last shape, where the Khmer word
//! is on the left and all its spellings on the right. Rows that cannot be used
//! are dropped and counted by reason, so nothing disappears silently.
//!
//! `--freq FILE` gives a frequency list (`Khmer word <TAB> count`). A word
//! found there gets its real share of the counts; the others get `--count`.

use std::collections::{BTreeMap, HashMap};
use std::process::ExitCode;

use khmer_core::{is_valid, normalize};

/// The count per million given to imported words that have no known
/// frequency, when `--count` is not used. Low, because most words of an
/// outside list are rare.
const DEFAULT_COUNT: f32 = 20.0;
/// No word gets a count per million below this.
const LOWEST_COUNT: f32 = 0.1;

pub fn run(args: &[String]) -> ExitCode {
    let mut paths = Vec::new();
    let mut count = DEFAULT_COUNT;
    let mut notes = Vec::new();
    let mut freq_path = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--count" => match args.next().and_then(|value| value.parse().ok()) {
                Some(value) if value > 0.0 => count = value,
                _ => return usage("--count needs a number above zero"),
            },
            "--freq" => match args.next() {
                Some(path) => freq_path = Some(path.clone()),
                None => return usage("--freq needs a file"),
            },
            "--note" => match args.next() {
                Some(note) => notes.push(note.clone()),
                None => return usage("--note needs some text"),
            },
            _ => paths.push(arg.clone()),
        }
    }
    // The last path is the output; the ones before it are read in order, so
    // put the most trusted source first: its spellings rank first.
    let Some((output, inputs)) = paths.split_last().filter(|(_, inputs)| !inputs.is_empty())
    else {
        return usage("import needs at least one input file and an output file");
    };
    let mut text = String::new();
    for input in inputs {
        match std::fs::read_to_string(input) {
            Ok(more) => {
                text.push_str(&more);
                text.push('\n');
            }
            Err(error) => {
                eprintln!("could not read {input}: {error}");
                return ExitCode::from(2);
            }
        }
    }
    let frequencies = match freq_path {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => read_frequencies(&text),
            Err(error) => {
                eprintln!("could not read {path}: {error}");
                return ExitCode::from(2);
            }
        },
        None => HashMap::new(),
    };

    // Word -> its spellings, in the order they were first seen.
    let mut order: Vec<String> = Vec::new();
    let mut spellings: HashMap<String, Vec<String>> = HashMap::new();
    let mut dropped: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut kept = 0;
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (word, results) = match read_line(line) {
            Ok(read) => read,
            Err(reason) => {
                *dropped.entry(reason).or_default() += 1;
                continue;
            }
        };
        for result in results {
            let spelling = match result {
                Ok(spelling) => spelling,
                Err(reason) => {
                    *dropped.entry(reason).or_default() += 1;
                    continue;
                }
            };
            let list = spellings.entry(word.clone()).or_insert_with(|| {
                order.push(word.clone());
                Vec::new()
            });
            if list.contains(&spelling) {
                *dropped.entry("the same pair again").or_default() += 1;
            } else {
                list.push(spelling);
                kept += 1;
            }
        }
    }

    let mut file = String::new();
    for note in &notes {
        file.push_str(&format!("# {note}\n"));
    }
    file.push_str("#\n# Made by `khmer import`. Do not edit by hand: fix the source or the\n");
    file.push_str("# importer and run it again. Put corrections in words.tsv, which wins.\n");
    file.push_str("#\n# Columns: Khmer word <TAB> count per million <TAB> spellings\n\n");
    let mut with_frequency = 0;
    for word in &order {
        let count = match frequencies.get(word) {
            Some(&per_million) => {
                with_frequency += 1;
                per_million.max(LOWEST_COUNT)
            }
            None => count,
        };
        // Two decimals are plenty; drop a trailing ".00".
        let count = format!("{count:.2}");
        let count = count.trim_end_matches('0').trim_end_matches('.');
        file.push_str(&format!("{word}\t{count}\t{}\n", spellings[word].join(", ")));
    }
    if let Err(error) = std::fs::write(output, file) {
        eprintln!("could not write {output}: {error}");
        return ExitCode::from(2);
    }

    println!("kept     {kept} pairs for {} words", order.len());
    if !frequencies.is_empty() {
        let unspelled = frequencies.keys().filter(|word| !spellings.contains_key(*word)).count();
        println!("         {with_frequency} of those words have a real frequency");
        println!("         {unspelled} words of the frequency list have no spelling and were left out");
    }
    for (reason, rows) in &dropped {
        println!("dropped  {rows:>6}  {reason}");
    }
    println!("wrote    {output}");
    ExitCode::SUCCESS
}

fn usage(problem: &str) -> ExitCode {
    eprintln!(
        "{problem}\n\nusage: khmer import PAIRS_FILE... OUT_FILE [--freq FILE] [--count N] [--note TEXT]..."
    );
    ExitCode::from(2)
}

/// Reads a frequency list (`Khmer word <TAB> count`) into counts per million.
fn read_frequencies(text: &str) -> HashMap<String, f32> {
    let mut counts: HashMap<String, f64> = HashMap::new();
    for line in text.lines() {
        let mut columns = line.split('\t');
        let word = normalize(columns.next().unwrap_or("").trim());
        let count: f64 = columns.next().and_then(|c| c.trim().parse().ok()).unwrap_or(0.0);
        if count > 0.0 && is_word(&word).is_ok() {
            // The same word can appear under two typing orders: add them up.
            *counts.entry(word).or_default() += count;
        }
    }
    let total: f64 = counts.values().sum();
    counts
        .into_iter()
        .map(|(word, count)| (word, (count * 1_000_000.0 / total) as f32))
        .collect()
}

/// The result of reading one spelling: the clean spelling, or why not.
type Spelling = Result<String, &'static str>;

/// Reads one line into a normalized Khmer word and its spellings, or says why
/// the line cannot be used.
fn read_line(line: &str) -> Result<(String, Vec<Spelling>), &'static str> {
    // "word=spelling,spelling": several spellings for one word.
    if let Some((khmer, latin)) = line.split_once('=') {
        let word = normalize(khmer.trim());
        is_word(&word)?;
        return Ok((word, latin.split(',').map(clean_spelling).collect()));
    }
    let (word, spelling) = read_pair(line)?;
    Ok((word, vec![Ok(spelling)]))
}

/// Reads a line holding one pair into (normalized Khmer word, clean Latin
/// spelling), or says why the line cannot be used.
fn read_pair(line: &str) -> Result<(String, String), &'static str> {
    let separator = ['\t', ':', ',']
        .into_iter()
        .find(|separator| line.contains(*separator))
        .ok_or("no separator between the two sides")?;
    let mut fields = line.split(separator).map(str::trim);
    let (first, second) = (fields.next().unwrap_or(""), fields.next().unwrap_or(""));
    // Either side may be the Khmer one.
    let (khmer, latin) =
        if first.chars().any(is_khmer_letter) { (first, second) } else { (second, first) };

    let word = normalize(khmer);
    is_word(&word)?;
    Ok((word, clean_spelling(latin)?))
}

/// Checks that a normalized string is a Khmer word the engine can store.
fn is_word(word: &str) -> Result<(), &'static str> {
    if word.is_empty() || !word.chars().all(is_khmer_letter) {
        // Mixed scripts, digits, punctuation, or the repeat sign ៗ, which the
        // Latin side does not show.
        return Err("the Khmer side is not only Khmer letters");
    }
    if !is_valid(word) {
        return Err("the Khmer side is not a valid syllable sequence");
    }
    Ok(())
}

/// Cleans one Latin spelling, or says why it cannot be typed.
fn clean_spelling(latin: &str) -> Spelling {
    let latin = latin.trim();
    if latin.contains(char::is_whitespace) {
        return Err("the Latin side has a space");
    }
    // "k'nhom" can only be typed as "knhom": an apostrophe ends the token.
    let spelling: String = latin
        .chars()
        .filter(|c| !matches!(c, '\'' | '\u{2019}'))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if spelling.is_empty() || !spelling.chars().all(|c| c.is_ascii_lowercase()) {
        return Err("the Latin side is not only letters a-z");
    }
    Ok(spelling)
}

/// Khmer consonants, vowels and signs: no digits, punctuation or symbols.
fn is_khmer_letter(c: char) -> bool {
    matches!(c, '\u{1780}'..='\u{17D3}' | '\u{17DD}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_usual_shapes() {
        let pair = Ok(("ខ្ញុំ".to_string(), "knhom".to_string()));
        assert_eq!(read_pair("ខ្ញុំ: knhom"), pair);
        assert_eq!(read_pair("ខ្ញុំ\tknhom"), pair);
        assert_eq!(read_pair("knhom,ខ្ញុំ,my-chats,own,real"), pair);
    }

    #[test]
    fn reads_several_spellings_after_an_equals_sign() {
        let (word, spellings) = read_line("ជា=chea,jea, je a").unwrap();
        assert_eq!(word, "ជា");
        assert_eq!(spellings[0], Ok("chea".to_string()));
        assert_eq!(spellings[1], Ok("jea".to_string()));
        assert!(spellings[2].is_err());
    }

    #[test]
    fn frequencies_become_counts_per_million() {
        let counts = read_frequencies("បាន\t3\nការ\t1\nnot khmer\t9\n");
        assert_eq!(counts.len(), 2);
        assert_eq!(counts["បាន"], 750_000.0);
        assert_eq!(counts["ការ"], 250_000.0);
    }

    #[test]
    fn cleans_the_latin_side() {
        assert_eq!(read_pair("ខ្ញុំ: K'nhom").unwrap().1, "knhom");
    }

    #[test]
    fn normalizes_the_khmer_side() {
        // ស + ី + ្រ + ្ត is stored in the right order.
        let wrong_order = "\u{179F}\u{17B8}\u{17D2}\u{179A}\u{17D2}\u{178F}: satrey";
        assert_eq!(
            read_pair(wrong_order).unwrap().0,
            "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"
        );
    }

    #[test]
    fn drops_what_cannot_be_typed_or_stored() {
        assert!(read_pair("កូស្តារីកា: Costa Rica").unwrap_err().contains("space"));
        assert!(read_pair("ស្រិចៗ: srech").unwrap_err().contains("only Khmer letters"));
        assert!(read_pair("អូខេ: o.k").unwrap_err().contains("a-z"));
        assert!(read_pair("\u{17B6}: a").unwrap_err().contains("valid"));
        assert!(read_pair("just some words").unwrap_err().contains("separator"));
    }
}
