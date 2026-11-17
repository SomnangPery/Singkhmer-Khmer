mod import;

use std::fs::File;
use std::io::{BufRead, Read, Write};
use std::process::ExitCode;
use std::sync::Mutex;

use khmer_core::{normalize, validate, Engine, Mode, Romanizer};

static OUT: Mutex<Option<File>> = Mutex::new(None);

macro_rules! say {
    ($($arg:tt)*) => {
        say_line(&format!($($arg)*))
    };
}

fn say_line(line: &str) {
    let mut out = OUT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match out.as_mut() {
        Some(file) => {
            if let Err(error) = writeln!(file, "{line}") {
                eprintln!("could not write to the --out file: {error}");
            }
        }
        None => println!("{line}"),
    }
}

const USAGE: &str = "\
usage:
  khmer normalize [--hex] [TEXT]   put TEXT into the correct stored order
  khmer check     [--hex] [TEXT]   say whether TEXT is stored correctly
  khmer type      [--hex] [KEYS]   type KEYS in Khmer mode, show what reaches the app
  khmer roman     [TEXT]           turn romanized Khmer into Khmer, top 3 suggestions
  khmer keys      [KEYS]           type KEYS in romanized mode, one key at a time
  khmer accuracy  FILE             measure romanized accuracy on a file of sentences
  khmer import    PAIRS... OUT     turn files of Khmer/Latin pairs into a words file
                                   options: --freq FILE, --count N, --note TEXT

With no TEXT, input is read from standard input.
`khmer roman` with no TEXT converts each line you type; Ctrl+Z then Enter ends it.
--hex        the input is code points, for example: 179F 17B8 17D2 179A 17D2 178F
--out FILE   write the results to FILE instead of the screen. Terminals cannot
             draw Khmer properly; open FILE in an editor to read it.";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let hex = match args.iter().position(|a| a == "--hex") {
        Some(position) => {
            args.remove(position);
            true
        }
        None => false,
    };
    if let Some(position) = args.iter().position(|a| a == "--out") {
        args.remove(position);
        if position >= args.len() {
            eprintln!("--out needs a file name\n\n{USAGE}");
            return ExitCode::from(2);
        }
        let path = args.remove(position);
        match File::create(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all("\u{FEFF}".as_bytes()) {
                    eprintln!("could not write to {path}: {error}");
                    return ExitCode::from(2);
                }
                *OUT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(file);
                eprintln!("writing results to {path}");
            }
            Err(error) => {
                eprintln!("could not create {path}: {error}");
                return ExitCode::from(2);
            }
        }
    }
    if args.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let command = args.remove(0);

    if command == "roman" && args.is_empty() {
        return roman_lines();
    }
    if command == "accuracy" {
        return accuracy(&args);
    }
    if command == "import" {
        return import::run(&args);
    }

    let raw = if args.is_empty() {
        let mut input = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut input) {
            eprintln!("could not read standard input: {error}");
            return ExitCode::from(2);
        }
        input
    } else {
        args.join(" ")
    };
    let text = if hex {
        match from_hex(&raw) {
            Ok(text) => text,
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            }
        }
    } else {
        raw.trim_end_matches(['\r', '\n']).to_string()
    };

    match command.as_str() {
        "normalize" => {
            let result = normalize(&text);
            show("input ", &text);
            show("output", &result);
            if result == text {
                say!("already in the correct order");
            }
            ExitCode::SUCCESS
        }
        "check" => {
            show("input ", &text);
            match validate(&text) {
                Ok(()) => {
                    say!("valid");
                    ExitCode::SUCCESS
                }
                Err(problem) => {
                    say!("not valid: {problem}");
                    ExitCode::FAILURE
                }
            }
        }
        "type" => {
            let mut engine = Engine::new();
            let mut app = String::new();
            for key in text.chars() {
                app.push_str(&engine.on_key(key).commit);
            }
            app.push_str(&engine.commit().commit);
            show("keys  ", &text);
            show("stored", &app);
            ExitCode::SUCCESS
        }
        "roman" => {
            roman(&text);
            ExitCode::SUCCESS
        }
        "keys" => {
            keys(&text);
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("unknown command: {command}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn roman(line: &str) {
    let suggestions = Romanizer::builtin().convert(line);
    if suggestions.is_empty() {
        return;
    }
    say!("typed  {}", line.trim());
    for (number, suggestion) in suggestions.iter().enumerate() {
        let words: Vec<&str> = suggestion.words.iter().map(|word| word.text.as_str()).collect();
        say!("  {}.   {}    ({})", number + 1, suggestion.text(), words.join(" + "));
    }
}

fn roman_lines() -> ExitCode {
    for line in std::io::stdin().lock().lines() {
        match line {
            Ok(line) => roman(line.trim_start_matches('\u{FEFF}')),
            Err(error) => {
                eprintln!("could not read standard input: {error}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

fn keys(text: &str) {
    let mut engine = Engine::new();
    engine.set_mode(Mode::Romanized);
    let mut app = String::new();
    for key in text.chars() {
        let output = engine.on_key(key);
        app.push_str(&output.commit);
        let name = if key == ' ' {
            "Space".to_string()
        } else {
            key.to_string()
        };
        say!(
            "{name:<6} app: {app:<20} typing: {:<24} suggestions: {}",
            output.preedit,
            engine.candidates().join(" | ")
        );
    }
    app.push_str(&engine.commit().commit);
    say!("end    app: {app}");
}

fn accuracy(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("accuracy needs a file of sentences\n\n{USAGE}");
        return ExitCode::from(2);
    };
    let file = match std::fs::read_to_string(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("could not read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let (mut total, mut first, mut first_three) = (0, 0, 0);
    for line in file.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((typed, expected)) = line.split_once('\t') else {
            eprintln!("skipped a line with no TAB: {line}");
            continue;
        };
        let expected = normalize(expected.trim());
        let suggestions: Vec<String> = Romanizer::builtin().convert(typed).iter().map(|s| s.text()).collect();
        total += 1;
        match suggestions.iter().position(|text| *text == expected) {
            Some(0) => {
                first += 1;
                first_three += 1;
            }
            Some(place) => {
                first_three += 1;
                say!("suggestion {}: {}", place + 1, typed.trim());
                say!("    first was  {}", suggestions[0]);
            }
            None => {
                say!("missing: {}", typed.trim());
                say!("    wanted  {expected}");
                say!("    got     {}", suggestions.join(" | "));
            }
        }
    }
    if total == 0 {
        eprintln!("{path} has no sentences");
        return ExitCode::from(2);
    }
    let percent = |count: i32| 100.0 * count as f32 / total as f32;
    say!("sentences         {total}");
    say!("first suggestion  {first} ({:.0}%)", percent(first));
    say!("first three       {first_three} ({:.0}%)", percent(first_three));
    ExitCode::SUCCESS
}

fn show(label: &str, text: &str) {
    let codes: Vec<String> = text.chars().map(|c| format!("{:04X}", c as u32)).collect();
    say!("{label}  {text}");
    say!("        {}", codes.join(" "));
}

fn from_hex(input: &str) -> Result<String, String> {
    input
        .split_whitespace()
        .map(|word| {
            let digits = word.trim_start_matches("U+").trim_start_matches("u+");
            u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| format!("not a code point: {word}"))
        })
        .collect()
}
