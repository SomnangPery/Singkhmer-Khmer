//! Property tests: rules that must hold for any input at all, checked on
//! thousands of random strings.

use khmer_core::{is_valid, normalize, normalize_with, Engine, Mode, Options, Romanizer, BACKSPACE};
use proptest::prelude::*;

/// A random character, weighted towards the ones that interact.
fn any_key() -> impl Strategy<Value = char> {
    prop_oneof![
        // Any character of the Khmer block.
        6 => (0x1780u32..=0x17DDu32).prop_map(|code| char::from_u32(code).unwrap()),
        // Extra coeng signs, so subscripts are common.
        3 => Just('\u{17D2}'),
        // Shifters and the vowels that interact with them.
        2 => prop_oneof![
            Just('\u{17C9}'), Just('\u{17CA}'), Just('\u{17BB}'), Just('\u{17B6}'),
            Just('\u{17B8}'), Just('\u{17C1}'), Just('\u{17C6}'), Just('\u{17D0}'),
        ],
        // Zero width non-joiner and joiner.
        1 => prop_oneof![Just('\u{200C}'), Just('\u{200D}')],
        // Things that are not Khmer letters.
        1 => prop_oneof![Just(' '), Just('a'), Just('\u{200B}'), Just('\u{17E1}'), Just('\u{17D4}')],
    ]
}

fn any_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(any_key(), 0..16).prop_map(|chars| chars.into_iter().collect())
}

/// Random key presses, including backspace.
fn any_typing() -> impl Strategy<Value = Vec<char>> {
    proptest::collection::vec(prop_oneof![9 => any_key(), 1 => Just(BACKSPACE)], 0..24)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    /// Normalizing twice gives the same result as normalizing once.
    #[test]
    fn normalize_is_idempotent(text in any_text()) {
        let once = normalize(&text);
        prop_assert_eq!(normalize(&once), once);
    }

    /// Text that is already valid is not changed, apart from the two
    /// substitutions that are replacements rather than reordering: coeng da
    /// (turned off here) and the deprecated letters (skipped here).
    #[test]
    fn valid_text_is_left_alone(text in any_text()) {
        let has_deprecated = text.contains('\u{17A3}') || text.contains('\u{17A4}');
        if is_valid(&text) && !has_deprecated {
            let keep_da = Options { fold_coeng_da: false };
            prop_assert_eq!(normalize_with(&text, keep_da), text);
        }
    }

    /// Normalizing never makes text longer: the only length changes are
    /// merged lookalikes and collapsed coeng signs. The one exception is the
    /// deprecated letter ឤ, which becomes the two characters អា.
    #[test]
    fn normalize_never_grows_text(text in any_text()) {
        let allowed_growth = text.matches('\u{17A4}').count();
        let before = text.chars().count();
        let after = normalize(&text).chars().count();
        prop_assert!(after <= before + allowed_growth);
    }

    /// Whatever keys are pressed in Khmer mode, in whatever order, the text
    /// that reaches the app is valid and already normalized.
    #[test]
    fn typed_text_is_always_valid(keys in any_typing()) {
        let mut engine = Engine::new();
        let mut app = String::new();
        for key in keys {
            app.push_str(&engine.on_key(key).commit);
        }
        app.push_str(&engine.commit().commit);

        prop_assert!(is_valid(&app), "invalid: {:?}", app);
        prop_assert_eq!(normalize(&app), app);
    }
}

/// A random key press in romanized mode: mostly letters and spaces.
fn any_romanized_key() -> impl Strategy<Value = char> {
    prop_oneof![
        // Letters that start or continue many dictionary spellings.
        10 => proptest::sample::select("nhjgtvkbsaoeiumrlypd".chars().collect::<Vec<char>>()),
        // Any letter, either case.
        3 => proptest::char::range('a', 'z'),
        1 => proptest::char::range('A', 'Z'),
        5 => Just(' '),
        2 => Just(BACKSPACE),
        // Punctuation, digits, a new line.
        2 => prop_oneof![Just('.'), Just('?'), Just('7'), Just('\n'), Just('\'')],
        // Khmer characters arriving while in romanized mode.
        1 => any_key(),
    ]
}

/// One romanized token: either a real dictionary spelling or random letters.
fn any_token() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::sample::select(vec![
            "nh", "jg", "tv", "sala", "rean", "ot", "te", "hz", "nv", "bong", "oun", "ey", "pros",
            "kon", "ok", "facebook", "sal", "khnhom", "b", "o",
        ])
        .prop_map(str::to_string),
        "[a-z]{1,6}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Whatever keys are pressed in romanized mode, nothing crashes and the
    /// text that reaches the app is valid and already normalized.
    #[test]
    fn romanized_text_is_always_valid(keys in proptest::collection::vec(any_romanized_key(), 0..40)) {
        let mut engine = Engine::new();
        engine.set_mode(Mode::Romanized);
        let mut app = String::new();
        for key in keys {
            let output = engine.on_key(key);
            // Something is shown exactly while something is being typed.
            prop_assert_eq!(output.preedit.is_empty(), !engine.is_composing());
            prop_assert_eq!(engine.candidates().is_empty(), !engine.is_composing());
            app.push_str(&output.commit);
        }
        app.push_str(&engine.commit().commit);

        prop_assert!(is_valid(&app), "invalid: {:?}", app);
        prop_assert_eq!(normalize(&app), app);
    }

    /// Typing a short phrase key by key gives the same text as converting the
    /// whole line at once: Space and commit use the top suggestion.
    #[test]
    fn typing_matches_converting_the_line(tokens in proptest::collection::vec(any_token(), 1..7)) {
        let line = tokens.join(" ");
        let expected = Romanizer::builtin().convert(&line)[0].text();

        let mut engine = Engine::new();
        engine.set_mode(Mode::Romanized);
        let mut app = String::new();
        for key in line.chars() {
            app.push_str(&engine.on_key(key).commit);
        }
        prop_assert_eq!(app.as_str(), "", "committed before the phrase was finished");
        prop_assert_eq!(engine.candidates()[0].clone(), expected.clone());
        prop_assert_eq!(engine.commit().commit, expected);
    }
}
