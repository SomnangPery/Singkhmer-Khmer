//! The engine as a native library for the Android app.
//!
//! Each function here is one `external` method of the Kotlin class
//! `com.newkeyboard.khmer.NativeEngine` (android/app/src/main/java/...). The
//! names must match that class: `Java_<package>_<class>_<method>`.
//!
//! This file only passes values across; all Khmer logic is in `khmer-core`.
//!
//! An engine is handed to Kotlin as a number (a pointer). Kotlin gives it back
//! on every call and calls `nativeDestroy` once when it is done with it. The
//! engine is not thread-safe, so the app calls it from one thread only.
//!
//! Calls that produce text return three strings: the text to commit, the text
//! still being typed, and "1" or "0" for whether the engine handled the key.

use std::panic::{catch_unwind, AssertUnwindSafe};

use jni::objects::{JClass, JObject, JObjectArray};
use jni::sys::{jint, jlong, jobjectArray};
use jni::JNIEnv;
use khmer_core::{Engine, Mode, Output, Romanizer};

/// The mode numbers Kotlin uses.
const MODE_ROMANIZED: jint = 1;

/// Loads the dictionary. The first key press in romanized mode would do this
/// by itself, but it takes a moment, so the app calls it once in the
/// background when the keyboard starts.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeWarmUp<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) {
    let _ = catch_unwind(|| {
        Romanizer::builtin();
    });
}

#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeCreate<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
) -> jlong {
    Box::into_raw(Box::new(Engine::new())) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeDestroy<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    if handle != 0 {
        // Safety: the handle came from nativeCreate and Kotlin never uses it
        // again after this call.
        drop(unsafe { Box::from_raw(handle as *mut Engine) });
    }
}

/// One key press. `key` is a Unicode code point; 8 is Backspace.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeOnKey<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    key: jint,
) -> jobjectArray {
    let output = with_engine(handle, |engine| {
        match u32::try_from(key).ok().and_then(char::from_u32) {
            Some(key) => engine.on_key(key),
            // Not a character: nothing to do, and nothing for the app to do either.
            None => Output { preedit: engine.preedit(), handled: true, ..Output::default() },
        }
    });
    output_array(&mut env, output)
}

/// Switches mode: 0 is Khmer, 1 is romanized.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeSetMode<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    mode: jint,
) -> jobjectArray {
    let mode = if mode == MODE_ROMANIZED { Mode::Romanized } else { Mode::Khmer };
    let output = with_engine(handle, |engine| engine.set_mode(mode));
    output_array(&mut env, output)
}

/// Finishes whatever is being typed.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeCommit<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jobjectArray {
    let output = with_engine(handle, Engine::commit);
    output_array(&mut env, output)
}

/// Picks suggestion number `index`.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeSelect<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    index: jint,
) -> jobjectArray {
    let output = with_engine(handle, |engine| match usize::try_from(index) {
        Ok(index) => engine.select(index),
        Err(_) => Output { preedit: engine.preedit(), handled: true, ..Output::default() },
    });
    output_array(&mut env, output)
}

/// Throws away whatever is being typed.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeReset<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    with_engine(handle, Engine::reset);
}

/// The suggestions, best first.
#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeCandidates<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jobjectArray {
    let candidates = with_engine(handle, |engine| engine.candidates());
    let candidates: Vec<&str> = candidates.iter().flatten().map(String::as_str).collect();
    string_array(&mut env, &candidates)
}

/// Runs `action` on the engine behind `handle`. None when the handle is not an
/// engine, or when the engine panicked: a panic must not cross into Java, where
/// it would close whichever app the keyboard is typing into.
fn with_engine<T>(handle: jlong, action: impl FnOnce(&mut Engine) -> T) -> Option<T> {
    if handle == 0 {
        return None;
    }
    // Safety: the handle came from nativeCreate, has not been destroyed, and
    // the app uses one engine from one thread only.
    let engine = unsafe { &mut *(handle as *mut Engine) };
    catch_unwind(AssertUnwindSafe(|| action(engine))).ok()
}

/// `[commit, preedit, "1" or "0"]`, or null when there is no output.
fn output_array(env: &mut JNIEnv, output: Option<Output>) -> jobjectArray {
    match output {
        Some(output) => {
            let handled = if output.handled { "1" } else { "0" };
            string_array(env, &[&output.commit, &output.preedit, handled])
        }
        None => std::ptr::null_mut(),
    }
}

/// A Java `String[]`, or null if Java could not make one.
fn string_array(env: &mut JNIEnv, items: &[&str]) -> jobjectArray {
    let made = (|| -> jni::errors::Result<JObjectArray> {
        let array = env.new_object_array(items.len() as jint, "java/lang/String", JObject::null())?;
        for (index, item) in items.iter().enumerate() {
            let string = env.new_string(item)?;
            env.set_object_array_element(&array, index as jint, &string)?;
            env.delete_local_ref(string)?;
        }
        Ok(array)
    })();
    match made {
        Ok(array) => array.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
