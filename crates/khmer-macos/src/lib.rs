//! The engine for the Mac input method, as plain C functions. The Swift side
//! (`macos/Sources/Engine.swift`) calls them through `macos/Sources/khmer.h`;
//! keep the two in step.
//!
//! Every string handed out is UTF-8, ends in NUL, and belongs to the caller,
//! who gives it back with `bk_free` (or `bk_output_free`). A list is one
//! string with an item per line. A panic inside the engine never crosses into
//! Swift: the call returns an empty answer instead.

use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

use khmer_core::{parse_user_word, set_user_words, Engine, Mode, Output, Romanizer};

/// The engine, opaque to Swift.
pub struct BKEngine(Engine);

#[repr(C)]
pub struct BKOutput {
    /// Text for the app, to insert in place of the marked text.
    pub commit: *mut c_char,
    /// What to show marked (underlined) after it, while the phrase is typed.
    pub preedit: *mut c_char,
    /// Whether the key was the keyboard's. When false, the app gets the key.
    pub handled: bool,
}

fn owned(text: &str) -> *mut c_char {
    // The engine's text never holds NUL; drop one if it ever does.
    let text: String = text.chars().filter(|c| *c != '\0').collect();
    CString::new(text).map(CString::into_raw).unwrap_or(std::ptr::null_mut())
}

fn borrowed(text: *const c_char) -> String {
    if text.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
}

fn output(answer: Option<Output>) -> BKOutput {
    let answer = answer.unwrap_or_default();
    BKOutput { commit: owned(&answer.commit), preedit: owned(&answer.preedit), handled: answer.handled }
}

fn with_engine<T: Default>(engine: *mut BKEngine, action: impl FnOnce(&mut Engine) -> T) -> T {
    if engine.is_null() {
        return T::default();
    }
    let engine = unsafe { &mut (*engine).0 };
    catch_unwind(AssertUnwindSafe(|| action(engine))).unwrap_or_default()
}

/// Reads the dictionary now, so the first key does not wait for it. Takes
/// about a second; call it off the main thread.
#[no_mangle]
pub extern "C" fn bk_warm_up() {
    let _ = catch_unwind(|| {
        Romanizer::builtin();
    });
}

/// A new engine in Singkhmer (romanized) mode.
#[no_mangle]
pub extern "C" fn bk_engine_new() -> *mut BKEngine {
    let mut engine = Engine::new();
    engine.set_mode(Mode::Romanized);
    Box::into_raw(Box::new(BKEngine(engine)))
}

#[no_mangle]
pub extern "C" fn bk_engine_free(engine: *mut BKEngine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

/// One key: a Latin letter, a space, 8 for Backspace, or any other character.
#[no_mangle]
pub extern "C" fn bk_on_key(engine: *mut BKEngine, key: u32) -> BKOutput {
    output(with_engine(engine, |engine| {
        Some(match char::from_u32(key) {
            Some(key) => engine.on_key(key),
            None => Output { preedit: engine.preedit(), handled: true, ..Output::default() },
        })
    }))
}

/// Sends suggestion `index` (0 is the top one).
#[no_mangle]
pub extern "C" fn bk_select(engine: *mut BKEngine, index: u32) -> BKOutput {
    output(with_engine(engine, |engine| Some(engine.select(index as usize))))
}

/// Sends the phrase as it is shown.
#[no_mangle]
pub extern "C" fn bk_commit(engine: *mut BKEngine) -> BKOutput {
    output(with_engine(engine, |engine| Some(engine.commit())))
}

/// Throws the phrase away, and forgets the words before it.
#[no_mangle]
pub extern "C" fn bk_reset(engine: *mut BKEngine) {
    with_engine(engine, Engine::reset);
}

#[no_mangle]
pub extern "C" fn bk_is_composing(engine: *mut BKEngine) -> bool {
    with_engine(engine, |engine| engine.is_composing())
}

/// The suggestions for the phrase, best first, one per line.
#[no_mangle]
pub extern "C" fn bk_candidates(engine: *mut BKEngine) -> *mut c_char {
    owned(&with_engine(engine, |engine| engine.candidates().join("\n")))
}

/// The Latin letters typed for the phrase ("nh jg tv").
#[no_mangle]
pub extern "C" fn bk_typed(engine: *mut BKEngine) -> *mut c_char {
    owned(&with_engine(engine, |engine| engine.typed()))
}

/// The corrections made since the last call, one "spelling = word" per line.
#[no_mangle]
pub extern "C" fn bk_take_corrections(engine: *mut BKEngine) -> *mut c_char {
    let lines = with_engine(engine, |engine| {
        engine.take_corrections().iter().map(|(spelling, word)| format!("{spelling} = {word}\n")).collect::<String>()
    });
    owned(&lines)
}

/// Replaces the user's words with the contents of their my-words.txt.
/// Returns how many words were kept; lines that cannot be read are skipped.
#[no_mangle]
pub extern "C" fn bk_set_user_words(text: *const c_char) -> u32 {
    let text = borrowed(text);
    catch_unwind(|| set_user_words(&text).0 as u32).unwrap_or(0)
}

/// Checks one "spelling = Khmer word" line. NULL when it is fine, otherwise
/// what is wrong with it, for the My words window to show.
#[no_mangle]
pub extern "C" fn bk_check_word(line: *const c_char) -> *mut c_char {
    let line = borrowed(line);
    match catch_unwind(|| parse_user_word(&line)) {
        Ok(Ok(_)) => std::ptr::null_mut(),
        Ok(Err(message)) => owned(&message),
        Err(_) => owned("the word could not be checked"),
    }
}

/// The best readings of a romanized phrase ("bong srey"), one per line.
#[no_mangle]
pub extern "C" fn bk_convert(input: *const c_char) -> *mut c_char {
    let input = borrowed(input);
    let readings = catch_unwind(|| {
        Romanizer::builtin().convert(&input).iter().map(|sentence| sentence.text()).collect::<Vec<_>>().join("\n")
    });
    owned(&readings.unwrap_or_default())
}

#[no_mangle]
pub extern "C" fn bk_free(text: *mut c_char) {
    if !text.is_null() {
        drop(unsafe { CString::from_raw(text) });
    }
}

#[no_mangle]
pub extern "C" fn bk_output_free(output: BKOutput) {
    bk_free(output.commit);
    bk_free(output.preedit);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(text: *mut c_char) -> String {
        let read = borrowed(text);
        bk_free(text);
        read
    }

    fn take_output(answer: BKOutput) -> (String, String, bool) {
        let read = (borrowed(answer.commit), borrowed(answer.preedit), answer.handled);
        bk_output_free(answer);
        read
    }

    fn type_keys(engine: *mut BKEngine, keys: &str) -> String {
        keys.chars().map(|key| take_output(bk_on_key(engine, key as u32)).0).collect()
    }

    #[test]
    fn a_phrase_typed_and_sent_with_a_second_space() {
        let engine = bk_engine_new();
        assert_eq!(type_keys(engine, "nh jg tv sala rean "), "");
        assert!(bk_is_composing(engine));
        assert_eq!(take(bk_typed(engine)), "nh jg tv sala rean");
        assert_eq!(take(bk_candidates(engine)).lines().next(), Some("ខ្ញុំចង់ទៅសាលារៀន"));
        assert_eq!(type_keys(engine, " "), "ខ្ញុំចង់ទៅសាលារៀន ");
        assert!(!bk_is_composing(engine));
        bk_engine_free(engine);
    }

    #[test]
    fn backspace_and_commit() {
        let engine = bk_engine_new();
        type_keys(engine, "nhx");
        let (commit, preedit, handled) = take_output(bk_on_key(engine, 8));
        assert_eq!((commit.as_str(), handled), ("", true));
        assert_eq!(preedit, "nh");
        assert_eq!(take_output(bk_commit(engine)).0, "ខ្ញុំ");
        assert!(!take_output(bk_on_key(engine, 8)).2, "Backspace with no phrase is the app's");
        bk_engine_free(engine);
    }

    #[test]
    fn picking_another_suggestion_gives_corrections() {
        let engine = bk_engine_new();
        type_keys(engine, "nh jg tv");
        let suggestions: Vec<String> = take(bk_candidates(engine)).lines().map(str::to_string).collect();
        let other = suggestions.iter().position(|s| *s != suggestions[0]).expect("a second suggestion");
        assert_eq!(take_output(bk_select(engine, other as u32)).0, suggestions[other]);
        let corrections = take(bk_take_corrections(engine));
        assert!(!corrections.is_empty() && corrections.lines().all(|line| line.contains(" = ")));
        assert_eq!(take(bk_take_corrections(engine)), "");
        bk_engine_free(engine);
    }

    #[test]
    fn words_are_checked_and_converted() {
        let good = CString::new("bong = បង").unwrap();
        assert!(bk_check_word(good.as_ptr()).is_null());
        let bad = CString::new("bo ng = បង").unwrap();
        assert!(take(bk_check_word(bad.as_ptr())).contains("a-z"));
        let phrase = CString::new("nh jg tv").unwrap();
        assert_eq!(take(bk_convert(phrase.as_ptr())).lines().next(), Some("ខ្ញុំចង់ទៅ"));
    }

    #[test]
    fn a_null_engine_answers_nothing() {
        let null = std::ptr::null_mut();
        assert_eq!(take_output(bk_on_key(null, 'a' as u32)), (String::new(), String::new(), false));
        assert_eq!(take(bk_candidates(null)), "");
        bk_reset(null);
        bk_engine_free(null);
    }
}
