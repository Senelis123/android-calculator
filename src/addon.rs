//! Bridge to the small Java add-on (classes.dex): widget, quick tile, floating window, voice input,
//! text-to-speech, TalkBack support, background update checks and Gemini Nano. Every call is optional:
//! if the Java class is missing or fails, the Rust app keeps working without that feature.

use crate::android::Jvm;
use crate::update::android::R;
use jni::objects::{JClass, JObject, JValue};
use jni::JNIEnv;

const CLASS: &str = "com.example.androidcalculator.rust.Addon";

/// Loads the add-on class through the activity's class loader (native threads only see system classes).
pub fn class<'a>(env: &mut JNIEnv<'a>, act: &JObject) -> R<JClass<'a>> {
    let loader = env.call_method(act, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?.l()?;
    let name = env.new_string(CLASS)?;
    let c = env.call_method(&loader, "loadClass", "(Ljava/lang/String;)Ljava/lang/Class;", &[JValue::Object(&name)])?.l()?;
    Ok(JClass::from(c))
}

/// Calls a static `(Context, String) -> String` method on the add-on; None when unavailable.
pub fn call(j: Jvm, method: &str, arg: &str) -> Option<String> {
    j.run(|env, act| -> R<Option<String>> {
        let c = class(env, act)?;
        let a = env.new_string(arg)?;
        let r = env.call_static_method(&c, method, "(Landroid/content/Context;Ljava/lang/String;)Ljava/lang/String;", &[JValue::Object(act), JValue::Object(&a)])?.l()?;
        if r.is_null() { return Ok(None); }
        Ok(Some(env.get_string(&r.into())?.into()))
    }).flatten()
}

pub fn speak(j: Jvm, text: &str) -> Option<String> { call(j, "speak", text) }
