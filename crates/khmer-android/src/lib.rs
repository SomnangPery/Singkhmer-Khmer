use std::panic::{catch_unwind, AssertUnwindSafe};

use jni::objects::{JClass, JObject, JObjectArray};
use jni::sys::{jint, jlong, jobjectArray};
use jni::JNIEnv;
use khmer_core::{Engine, Mode, Output, Romanizer};

const MODE_ROMANIZED: jint = 1;

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
        drop(unsafe { Box::from_raw(handle as *mut Engine) });
    }
}

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
            None => Output {
                preedit: engine.preedit(),
                handled: true,
                ..Output::default()
            },
        }
    });
    output_array(&mut env, output)
}

#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeSetMode<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    mode: jint,
) -> jobjectArray {
    let mode = if mode == MODE_ROMANIZED {
        Mode::Romanized
    } else {
        Mode::Khmer
    };
    let output = with_engine(handle, |engine| engine.set_mode(mode));
    output_array(&mut env, output)
}

#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeCommit<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) -> jobjectArray {
    let output = with_engine(handle, Engine::commit);
    output_array(&mut env, output)
}

#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeSelect<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    index: jint,
) -> jobjectArray {
    let output = with_engine(handle, |engine| match usize::try_from(index) {
        Ok(index) => engine.select(index),
        Err(_) => Output {
            preedit: engine.preedit(),
            handled: true,
            ..Output::default()
        },
    });
    output_array(&mut env, output)
}

#[no_mangle]
pub extern "system" fn Java_com_newkeyboard_khmer_NativeEngine_nativeReset<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
) {
    with_engine(handle, Engine::reset);
}

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

fn with_engine<T>(handle: jlong, action: impl FnOnce(&mut Engine) -> T) -> Option<T> {
    if handle == 0 {
        return None;
    }
    let engine = unsafe { &mut *(handle as *mut Engine) };
    catch_unwind(AssertUnwindSafe(|| action(engine))).ok()
}

fn output_array(env: &mut JNIEnv, output: Option<Output>) -> jobjectArray {
    match output {
        Some(output) => {
            let handled = if output.handled { "1" } else { "0" };
            string_array(env, &[&output.commit, &output.preedit, handled])
        }
        None => std::ptr::null_mut(),
    }
}

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
