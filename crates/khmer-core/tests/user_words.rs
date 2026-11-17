//! The user's own words. In a file of its own: the words are shared by the
//! whole program, and other tests must not see them.

use khmer_core::{set_user_words, Engine, Mode, Romanizer};

fn type_keys(keys: &str) -> Engine {
    let mut engine = Engine::new();
    engine.set_mode(Mode::Romanized);
    for key in keys.chars() {
        engine.on_key(key);
    }
    engine
}

#[test]
fn user_words_come_first_and_can_be_taken_away() {
    let before = Romanizer::builtin().convert("zzbong")[0].text();
    assert_eq!(before, "zzbong", "an unknown spelling stays in Latin letters");

    let (kept, skipped) = set_user_words("# mine\nzzbong = បង\nZZbong\tបងប្រុស\nnot a line\nhi = hi\n");
    assert_eq!(kept, 2);
    assert_eq!(skipped.iter().map(|error| error.line).collect::<Vec<_>>(), [4, 5]);

    let engine = type_keys("zzbong");
    assert_eq!(engine.typed(), "zzbong");
    assert_eq!(&engine.candidates()[..2], ["បង", "បងប្រុស"]);

    let engine = type_keys("nh zzbong");
    assert_eq!(engine.typed(), "nh zzbong");
    assert!(engine.candidates()[0].ends_with("បង"), "{:?}", engine.candidates());

    set_user_words("");
    assert_eq!(type_keys("zzbong").candidates()[0], "zzbong");
}
