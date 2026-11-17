use khmer_core::{is_valid, normalize, normalize_with, Engine, Mode, Options, Romanizer, BACKSPACE};
use proptest::prelude::*;

fn any_key() -> impl Strategy<Value = char> {
    prop_oneof![
        6 => (0x1780u32..=0x17DDu32).prop_map(|code| char::from_u32(code).unwrap()),
        3 => Just('\u{17D2}'),
        2 => prop_oneof![
            Just('\u{17C9}'), Just('\u{17CA}'), Just('\u{17BB}'), Just('\u{17B6}'),
            Just('\u{17B8}'), Just('\u{17C1}'), Just('\u{17C6}'), Just('\u{17D0}'),
        ],
        1 => prop_oneof![Just('\u{200C}'), Just('\u{200D}')],
        1 => prop_oneof![Just(' '), Just('a'), Just('\u{200B}'), Just('\u{17E1}'), Just('\u{17D4}')],
    ]
}

fn any_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(any_key(), 0..16).prop_map(|chars| chars.into_iter().collect())
}

fn any_typing() -> impl Strategy<Value = Vec<char>> {
    proptest::collection::vec(prop_oneof![9 => any_key(), 1 => Just(BACKSPACE)], 0..24)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn normalize_is_idempotent(text in any_text()) {
        let once = normalize(&text);
        prop_assert_eq!(normalize(&once), once);
    }

    #[test]
    fn valid_text_is_left_alone(text in any_text()) {
        let has_deprecated = text.contains('\u{17A3}') || text.contains('\u{17A4}');
        if is_valid(&text) && !has_deprecated {
            let keep_da = Options { fold_coeng_da: false };
            prop_assert_eq!(normalize_with(&text, keep_da), text);
        }
    }

    #[test]
    fn normalize_never_grows_text(text in any_text()) {
        let allowed_growth = text.matches('\u{17A4}').count();
        let before = text.chars().count();
        let after = normalize(&text).chars().count();
        prop_assert!(after <= before + allowed_growth);
    }

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

fn any_romanized_key() -> impl Strategy<Value = char> {
    prop_oneof![
        10 => proptest::sample::select("nhjgtvkbsaoeiumrlypd".chars().collect::<Vec<char>>()),
        3 => proptest::char::range('a', 'z'),
        1 => proptest::char::range('A', 'Z'),
        5 => Just(' '),
        2 => Just(BACKSPACE),
        2 => prop_oneof![Just('.'), Just('?'), Just('7'), Just('\n'), Just('\'')],
        1 => any_key(),
    ]
}

fn any_token() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::sample::select(vec![
            "nh", "jg", "tv", "sala", "rean", "ot", "te", "hz", "nv", "bong", "oun", "ey", "pros", "kon", "ok",
            "facebook", "sal", "khnhom", "b", "o",
        ])
        .prop_map(str::to_string),
        "[a-z]{1,6}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn romanized_text_is_always_valid(keys in proptest::collection::vec(any_romanized_key(), 0..40)) {
        let mut engine = Engine::new();
        engine.set_mode(Mode::Romanized);
        let mut app = String::new();
        for key in keys {
            let output = engine.on_key(key);
            prop_assert_eq!(output.preedit.is_empty(), !engine.is_composing());
            prop_assert_eq!(engine.candidates().is_empty(), !engine.is_composing());
            app.push_str(&output.commit);
        }
        app.push_str(&engine.commit().commit);

        prop_assert!(is_valid(&app), "invalid: {:?}", app);
        prop_assert_eq!(normalize(&app), app);
    }

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
