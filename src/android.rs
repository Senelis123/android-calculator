//! NativeActivity entry point: window drawing, touch, and platform effects via JNI. No Java code.

use crate::app::{self, Action, App, Effect, GalleryItem, PhotoState, Saved, Screen, PHOTO_SRC_W};
use crate::draw::{Canvas, Fonts};
use crate::update::android::{clear, http_get, Updater, R};
use crate::update::UpdateState;
use android_activity::input::{InputEvent, KeyAction, Keycode, MotionAction};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use jni::objects::{JObject, JValue};
use jni::{JNIEnv, JavaVM};
use ndk::hardware_buffer_format::HardwareBufferFormat;
use ndk::native_window::NativeWindow;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

enum Msg { Ask(Option<(String, bool)>), Nano(String), RateHist(String, String, Vec<(String, f64)>), Rates(Option<crate::convert::Rates>), Markets(crate::market::MarketKind, Vec<crate::market::Quote>), Gallery(Vec<GalleryItem>, String), Photo(PhotoState), Paste(String), Toast(String) }

#[derive(Clone, Copy)]
pub(crate) struct Jvm { pub vm: usize, pub act: usize }
impl Jvm {
    pub fn run<T>(&self, f: impl FnOnce(&mut JNIEnv, &JObject) -> R<T>) -> Option<T> {
        let vm = unsafe { JavaVM::from_raw(self.vm as *mut _) }.ok()?;
        let mut env = vm.attach_current_thread_permanently().ok()?;
        let act = unsafe { JObject::from_raw(self.act as jni::sys::jobject) };
        let r = env.with_local_frame(64, |env| f(env, &act));
        match r { Ok(v) => Some(v), Err(e) => { log::warn!("jni: {e}"); clear(&mut env); None } }
    }
}

fn service<'a>(env: &mut JNIEnv<'a>, act: &JObject, name: &str) -> R<JObject<'a>> {
    let n = env.new_string(name)?;
    env.call_method(act, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&n)])?.l()
}

fn ensure_looper(env: &mut JNIEnv) -> R<()> {
    let l = env.call_static_method("android/os/Looper", "myLooper", "()Landroid/os/Looper;", &[])?.l()?;
    if l.is_null() { env.call_static_method("android/os/Looper", "prepare", "()V", &[])?; }
    Ok(())
}

fn sdk(env: &mut JNIEnv) -> R<i32> { env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i() }

fn image_perm(env: &mut JNIEnv) -> R<&'static str> {
    Ok(if sdk(env)? >= 33 { "android.permission.READ_MEDIA_IMAGES" } else { "android.permission.READ_EXTERNAL_STORAGE" })
}

/// Returns true when the photo permission is granted; otherwise asks for it.
fn photo_permission(env: &mut JNIEnv, act: &JObject, ask: bool) -> R<bool> {
    let perm = image_perm(env)?; let p = env.new_string(perm)?;
    let g = env.call_method(act, "checkSelfPermission", "(Ljava/lang/String;)I", &[JValue::Object(&p)])?.i()?;
    if g == 0 { return Ok(true); }
    if ask {
        let arr = env.new_object_array(1, "java/lang/String", &p)?;
        env.call_method(act, "requestPermissions", "([Ljava/lang/String;I)V", &[JValue::Object(&arr), JValue::Int(7)])?;
    }
    Ok(false)
}

fn media_uri<'a>(env: &mut JNIEnv<'a>) -> R<JObject<'a>> {
    env.get_static_field("android/provider/MediaStore$Images$Media", "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;")?.l()
}

fn item_uri<'a>(env: &mut JNIEnv<'a>, id: i64) -> R<JObject<'a>> {
    let base = media_uri(env)?;
    env.call_static_method("android/content/ContentUris", "withAppendedId", "(Landroid/net/Uri;J)Landroid/net/Uri;", &[JValue::Object(&base), JValue::Long(id)])?.l()
}

fn bitmap_rgb(env: &mut JNIEnv, bmp: &JObject) -> R<image::RgbImage> {
    let w = env.call_method(bmp, "getWidth", "()I", &[])?.i()?;
    let h = env.call_method(bmp, "getHeight", "()I", &[])?.i()?;
    let arr = env.new_int_array(w * h)?;
    env.call_method(bmp, "getPixels", "([IIIIIII)V", &[JValue::Object(&arr), JValue::Int(0), JValue::Int(w), JValue::Int(0), JValue::Int(0), JValue::Int(w), JValue::Int(h)])?;
    let mut buf = vec![0i32; (w * h) as usize];
    env.get_int_array_region(&arr, 0, &mut buf)?;
    env.call_method(bmp, "recycle", "()V", &[])?;
    let mut img = image::RgbImage::new(w as u32, h as u32);
    for (p, c) in img.pixels_mut().zip(buf) { let c = c as u32; *p = image::Rgb([(c >> 16) as u8, (c >> 8) as u8, c as u8]); }
    Ok(img)
}

fn rotate(img: image::RgbImage, deg: i32) -> image::RgbImage {
    match deg.rem_euclid(360) { 90 => image::imageops::rotate90(&img), 180 => image::imageops::rotate180(&img), 270 => image::imageops::rotate270(&img), _ => img }
}

fn list_gallery(env: &mut JNIEnv, act: &JObject) -> R<Vec<GalleryItem>> {
    let resolver = env.call_method(act, "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
    let uri = media_uri(env)?;
    let cols = ["_id", "orientation", "date_added"];
    let proj = env.new_object_array(3, "java/lang/String", JObject::null())?;
    for (i, c) in cols.iter().enumerate() { let s = env.new_string(c)?; env.set_object_array_element(&proj, i as i32, s)?; }
    let order = env.new_string("date_added DESC")?;
    let cur = env.call_method(&resolver, "query", "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
        &[JValue::Object(&uri), JValue::Object(&proj), JValue::Object(&JObject::null()), JValue::Object(&JObject::null()), JValue::Object(&order)])?.l()?;
    let mut out = vec![];
    if cur.is_null() { return Ok(out); }
    let api = sdk(env)?;
    while out.len() < 24 && env.call_method(&cur, "moveToNext", "()Z", &[])?.z()? {
        let id = env.call_method(&cur, "getLong", "(I)J", &[JValue::Int(0)])?.j()?;
        let orient = env.call_method(&cur, "getInt", "(I)I", &[JValue::Int(1)])?.i()?;
        let thumb = env.with_local_frame(16, |env| -> R<Option<image::RgbImage>> {
            let bmp = if api >= 29 {
                let u = item_uri(env, id)?;
                let size = env.new_object("android/util/Size", "(II)V", &[JValue::Int(320), JValue::Int(320)])?;
                env.call_method(&resolver, "loadThumbnail", "(Landroid/net/Uri;Landroid/util/Size;Landroid/os/CancellationSignal;)Landroid/graphics/Bitmap;",
                    &[JValue::Object(&u), JValue::Object(&size), JValue::Object(&JObject::null())]).and_then(|v| v.l())
            } else {
                env.call_static_method("android/provider/MediaStore$Images$Thumbnails", "getThumbnail",
                    "(Landroid/content/ContentResolver;JILandroid/graphics/BitmapFactory$Options;)Landroid/graphics/Bitmap;",
                    &[JValue::Object(&resolver), JValue::Long(id), JValue::Int(1), JValue::Object(&JObject::null())]).and_then(|v| v.l())
            };
            match bmp {
                Ok(b) if !b.is_null() => { let img = bitmap_rgb(env, &b)?; Ok(Some(app::thumb(&if api >= 29 { img } else { rotate(img, orient) }, 320))) }
                _ => { clear(env); Ok(None) }
            }
        })?;
        out.push(GalleryItem { id, thumb, label: String::new() });
    }
    env.call_method(&cur, "close", "()V", &[])?;
    Ok(out)
}

fn open_stream<'a>(env: &mut JNIEnv<'a>, act: &JObject, id: i64) -> R<JObject<'a>> {
    let u = item_uri(env, id)?;
    let r = env.call_method(act, "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
    env.call_method(&r, "openInputStream", "(Landroid/net/Uri;)Ljava/io/InputStream;", &[JValue::Object(&u)])?.l()
}

/// Decodes a gallery photo scaled to at most ~2000 px on the long side, rotated upright.
fn load_photo(env: &mut JNIEnv, act: &JObject, id: i64) -> R<Option<image::RgbImage>> {
    let resolver = env.call_method(act, "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
    let uri = item_uri(env, id)?;
    let _ = (&resolver, &uri);
    let opts = env.new_object("android/graphics/BitmapFactory$Options", "()V", &[])?;
    env.set_field(&opts, "inJustDecodeBounds", "Z", JValue::Bool(1))?;
    let s = open_stream(env, act, id)?;
    env.call_static_method("android/graphics/BitmapFactory", "decodeStream", "(Ljava/io/InputStream;Landroid/graphics/Rect;Landroid/graphics/BitmapFactory$Options;)Landroid/graphics/Bitmap;",
        &[JValue::Object(&s), JValue::Object(&JObject::null()), JValue::Object(&opts)])?;
    env.call_method(&s, "close", "()V", &[])?;
    let w = env.get_field(&opts, "outWidth", "I")?.i()?; let h = env.get_field(&opts, "outHeight", "I")?.i()?;
    if w <= 0 || h <= 0 { return Ok(None); }
    let mut sample = 1; while w.max(h) / (sample * 2) >= 1600 { sample *= 2; }
    let opts = env.new_object("android/graphics/BitmapFactory$Options", "()V", &[])?;
    env.set_field(&opts, "inSampleSize", "I", JValue::Int(sample))?;
    let s = open_stream(env, act, id)?;
    let bmp = env.call_static_method("android/graphics/BitmapFactory", "decodeStream", "(Ljava/io/InputStream;Landroid/graphics/Rect;Landroid/graphics/BitmapFactory$Options;)Landroid/graphics/Bitmap;",
        &[JValue::Object(&s), JValue::Object(&JObject::null()), JValue::Object(&opts)])?.l()?;
    env.call_method(&s, "close", "()V", &[])?;
    if bmp.is_null() { return Ok(None); }
    let img = bitmap_rgb(env, &bmp)?;
    // EXIF orientation from MediaStore
    let mut deg = 0;
    let s = open_stream(env, act, id)?;
    if let Ok(ex) = env.new_object("android/media/ExifInterface", "(Ljava/io/InputStream;)V", &[JValue::Object(&s)]) {
        let tag = env.new_string("Orientation")?;
        let o = env.call_method(&ex, "getAttributeInt", "(Ljava/lang/String;I)I", &[JValue::Object(&tag), JValue::Int(1)])?.i()?;
        deg = match o { 6 => 90, 3 => 180, 8 => 270, _ => 0 };
    } else { clear(env); }
    let _ = env.call_method(&s, "close", "()V", &[]);
    Ok(Some(rotate(img, deg)))
}

fn effect(fx: Effect, app: &mut App, j: Jvm, tx: &Sender<Msg>, wake: &Arc<dyn Fn() + Send + Sync>, updater: &Updater, data: &std::path::Path, camera_pending: &mut bool) {
    let spawn = |f: Box<dyn FnOnce() -> Option<Msg> + Send>| { let tx = tx.clone(); let wake = wake.clone(); std::thread::spawn(move || { if let Some(m) = f() { let _ = tx.send(m); } wake(); }); };
    match fx {
        Effect::Vibrate => { let ms = app.s.vib_ms as i64; if ms > 0 { j.run(|env, act| { let v = service(env, act, "vibrator")?; if !v.is_null() { env.call_method(&v, "vibrate", "(J)V", &[JValue::Long(ms)])?; } Ok(()) }); } }
        Effect::Floating => { if crate::addon::call(j, "floating", "").as_deref() == Some("permission") { app.toast = Some(crate::i18n::t("Allow \"Display over other apps\", then tap again").into()); } }
        Effect::BgCheck(on) => { if on { j.run(|env, act| { if sdk(env)? >= 33 { let arr = env.new_object_array(1, "java/lang/String", jni::objects::JObject::null())?; let p = env.new_string("android.permission.POST_NOTIFICATIONS")?; env.set_object_array_element(&arr, 0, p)?; env.call_method(act, "requestPermissions", "([Ljava/lang/String;I)V", &[JValue::Object(&arr), JValue::Int(5)])?; } Ok(()) }); } let _ = crate::addon::call(j, "updateChecks", if on { "1" } else { "0" }); }
        Effect::Listen => { if crate::addon::call(j, "listen", crate::i18n::LANGS[crate::i18n::lang()]).is_some() { *camera_pending = false; app.toast = Some(crate::i18n::t("Listening…").into()); VOICE_PENDING.store(true, std::sync::atomic::Ordering::Relaxed); } else { app.toast = Some(crate::i18n::t("Voice input is not available on this phone").into()); } }
        Effect::NanoStatus => spawn(Box::new(move || Some(Msg::Nano(crate::addon::call(j, "nano", "status").unwrap_or_else(|| "unavailable".into()))))),
        Effect::Solve(text, use_nano) => spawn(Box::new(move || {
            if use_nano {
                let prompt = format!("Convert this math word problem into ONE arithmetic expression. Use only numbers, + - * / ^ ( ) and sqrt(). Reply with the expression only, no words.\nProblem: {text}");
                if let Some(e) = crate::addon::call(j, "nano", &format!("ask\t{prompt}")).and_then(|a| crate::expr::expr_from_model(&a)) { return Some(Msg::Ask(Some((e, true)))); }
            }
            Some(Msg::Ask(crate::expr::text_to_expr(&text).map(|e| (e, false))))
        })),
        Effect::Click => { j.run(|env, act| { let a = service(env, act, "audio")?; if !a.is_null() { env.call_method(&a, "playSoundEffect", "(IF)V", &[JValue::Int(0), JValue::Float(0.5)])?; } Ok(()) }); }
        Effect::SetIcon(i) => { let _ = crate::addon::call(j, "setIcon", &i.to_string()); }
        Effect::ReportBug => {
            let info = j.run(|env, _| -> R<String> {
                let f = |env: &mut JNIEnv, n: &str| -> R<String> { let o = env.get_static_field("android/os/Build", n, "Ljava/lang/String;")?.l()?; Ok(env.get_string(&o.into())?.into()) };
                let rel = { let o = env.get_static_field("android/os/Build$VERSION", "RELEASE", "Ljava/lang/String;")?.l()?; let s: String = env.get_string(&o.into())?.into(); s };
                Ok(format!("{} {} ({}), Android {}", f(env, "MANUFACTURER")?, f(env, "MODEL")?, f(env, "DEVICE")?, rel))
            }).unwrap_or_default();
            let body = format!("{}\n\n---\n{} {}\n{}\n{}: {}", crate::i18n::t("Describe the problem here:"), crate::i18n::t("Calculator"), crate::app::version(), info, crate::i18n::t("Language"), crate::i18n::LANGS[crate::i18n::lang()]);
            let url = format!("https://github.com/Senelis123/android-calculator/issues/new?title={}&body={}", enc(crate::i18n::t("Problem report")), enc(&body));
            j.run(|env, act| { let a = env.new_string("android.intent.action.VIEW")?; let u = env.new_string(&url)?;
                let uri = env.call_static_method("android/net/Uri", "parse", "(Ljava/lang/String;)Landroid/net/Uri;", &[JValue::Object(&u)])?.l()?;
                let i = env.new_object("android/content/Intent", "(Ljava/lang/String;Landroid/net/Uri;)V", &[JValue::Object(&a), JValue::Object(&uri)])?;
                env.call_method(act, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&i)])?; Ok(()) });
        }
        Effect::Copy(text) => { j.run(|env, act| {
            ensure_looper(env)?;
            let cm = service(env, act, "clipboard")?;
            let (l, t) = (env.new_string("Calculator")?, env.new_string(&text)?);
            let clip = env.call_static_method("android/content/ClipData", "newPlainText", "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;", &[JValue::Object(&l), JValue::Object(&t)])?.l()?;
            env.call_method(&cm, "setPrimaryClip", "(Landroid/content/ClipData;)V", &[JValue::Object(&clip)])?; Ok(())
        }); }
        Effect::Keyboard(_) => {}
        Effect::Speak(text) => { let _ = crate::addon::speak(j, &format!("{}\t{}", crate::i18n::LANGS[crate::i18n::lang()], text)); }
        Effect::Share(text) => { j.run(|env, act| {
            let action = env.new_string("android.intent.action.SEND")?;
            let intent = env.new_object("android/content/Intent", "(Ljava/lang/String;)V", &[JValue::Object(&action)])?;
            let ty = env.new_string("text/plain")?;
            env.call_method(&intent, "setType", "(Ljava/lang/String;)Landroid/content/Intent;", &[JValue::Object(&ty)])?;
            let (k, v) = (env.new_string("android.intent.extra.TEXT")?, env.new_string(&text)?);
            env.call_method(&intent, "putExtra", "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;", &[JValue::Object(&k), JValue::Object(&v)])?;
            let title = env.new_string(crate::i18n::t("Share"))?;
            let chooser = env.call_static_method("android/content/Intent", "createChooser", "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;", &[JValue::Object(&intent), JValue::Object(&title)])?.l()?;
            env.call_method(act, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&chooser)])?; Ok(())
        }); }
        Effect::SaveFile(name, content) => {
            let ok = j.run(|env, act| -> R<bool> {
                if sdk(env)? < 29 { return Ok(false); }
                let cv = env.new_object("android/content/ContentValues", "()V", &[])?;
                let put = |env: &mut JNIEnv, k: &str, v: &str| -> R<()> { let (k, v) = (env.new_string(k)?, env.new_string(v)?); env.call_method(&cv, "put", "(Ljava/lang/String;Ljava/lang/String;)V", &[JValue::Object(&k), JValue::Object(&v)])?; Ok(()) };
                put(env, "_display_name", &name)?;
                put(env, "mime_type", if name.ends_with(".csv") { "text/csv" } else { "application/json" })?;
                put(env, "relative_path", "Download/")?;
                let uri = env.get_static_field("android/provider/MediaStore$Downloads", "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;")?.l()?;
                let res = env.call_method(act, "getContentResolver", "()Landroid/content/ContentResolver;", &[])?.l()?;
                let item = env.call_method(&res, "insert", "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;", &[JValue::Object(&uri), JValue::Object(&cv)])?.l()?;
                if item.is_null() { return Ok(false); }
                let os = env.call_method(&res, "openOutputStream", "(Landroid/net/Uri;)Ljava/io/OutputStream;", &[JValue::Object(&item)])?.l()?;
                let bytes = env.byte_array_from_slice(content.as_bytes())?;
                env.call_method(&os, "write", "([B)V", &[JValue::Object(&bytes)])?;
                env.call_method(&os, "close", "()V", &[])?;
                Ok(true)
            }).unwrap_or(false);
            if !ok { let _ = tx.send(Msg::Toast(crate::i18n::t("Could not save the file").into())); }
        }
        Effect::RequestPaste => {
            let t = j.run(|env, act| -> R<Option<String>> {
                ensure_looper(env)?;
                let cm = service(env, act, "clipboard")?;
                let clip = env.call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[])?.l()?;
                if clip.is_null() { return Ok(None); }
                let item = env.call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[JValue::Int(0)])?.l()?;
                let cs = env.call_method(&item, "coerceToText", "(Landroid/content/Context;)Ljava/lang/CharSequence;", &[JValue::Object(act)])?.l()?;
                let s = env.call_method(&cs, "toString", "()Ljava/lang/String;", &[])?.l()?;
                Ok(Some(env.get_string(&s.into())?.into()))
            }).flatten();
            match t { Some(t) => { let _ = tx.send(Msg::Paste(t)); } None => { let _ = tx.send(Msg::Toast(crate::i18n::t("Clipboard is empty").into())); } }
        }
        Effect::Save => { if let Ok(js) = serde_json::to_string(&app.s) { let tmp = data.join("saved.json.tmp"); if std::fs::write(&tmp, js).is_ok() { let _ = std::fs::rename(&tmp, data.join("saved.json")); } } }
        Effect::FetchRates => { app.rates_status = crate::i18n::t("Updating rates…").into(); spawn(Box::new(move || {
            let body = j.run(|env, _| http_get(env, "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml")).flatten();
            Some(Msg::Rates(body.and_then(|b| crate::convert::parse_ecb(&b))))
        })); }
        Effect::FetchMarkets(kind) => {
            let kind2 = *kind;
            app.market_status = "Loading market data…".into();
            spawn(Box::new(move || {
                let quotes = match kind2 {
                    crate::market::MarketKind::Crypto => {
                        j.run(|env, _| http_get(env, &crate::market::crypto_url()))
                            .flatten()
                            .map(|body| crate::market::parse_crypto(&body))
                            .unwrap_or_default()
                    }
                    crate::market::MarketKind::Stocks => {
                        let mut out = Vec::new();
                        for (symbol, _, _) in crate::market::STOCKS {
                            if let Some(body) = j.run(|env, _| http_get(env, &crate::market::stock_url(*symbol))).flatten() {
                                if let Some(q) = crate::market::parse_stock(&body, *symbol) {
                                    out.push(q);
                                }
                            }
                        }
                        out
                    }
                };
                Some(Msg::Markets(kind2, quotes))
            }));
        }
        Effect::FetchRateHistory => {
            let c = app.s.conv_cat;
            let names = crate::convert::unit_names(c, &app.s.rates);
            let (f, t) = (names.get(app.s.conv_from[c]).cloned().unwrap_or_default(), names.get(app.s.conv_to[c]).cloned().unwrap_or_default());
            app.toast = Some(crate::i18n::t("Loading…").into());
            spawn(Box::new(move || {
                let body = j.run(|env, _| http_get(env, "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-hist-90d.xml")).flatten();
                Some(match body { Some(b) => Msg::RateHist(f.clone(), t.clone(), crate::convert::parse_history(&b, &f, &t)), None => Msg::Toast(crate::i18n::t("No internet connection").into()) })
            }));
        }
        Effect::CheckUpdate => updater.check(),
        Effect::UpdateTap => {
            let st = updater.state.lock().unwrap().clone();
            match st { UpdateState::Available { .. } => updater.start(), UpdateState::Failed { .. } | UpdateState::Installing { .. } => *updater.state.lock().unwrap() = UpdateState::Idle, _ => {} }
        }
        Effect::TakePhoto => {
            let ok = j.run(|env, act| { let a = env.new_string("android.media.action.STILL_IMAGE_CAMERA")?; let i = env.new_object("android/content/Intent", "(Ljava/lang/String;)V", &[JValue::Object(&a)])?;
                env.call_method(act, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&i)])?; Ok(()) }).is_some();
            if ok { *camera_pending = true; app.toast = Some(crate::i18n::t("Take the photo and come back - it will be in the list").into()); } else { app.toast = Some(crate::i18n::t("Could not open the camera").into()); }
        }
        Effect::OpenGallery => {
            if j.run(|env, act| photo_permission(env, act, true)) != Some(true) { app.gallery_status = crate::i18n::t("Allow the app to see photos and try again.").into(); return; }
            app.gallery_status = crate::i18n::t("Loading…").into();
            spawn(Box::new(move || { let items = j.run(|env, act| list_gallery(env, act)).unwrap_or_default();
                let st = if items.is_empty() { crate::i18n::t("No photos found").to_string() } else { String::new() }; Some(Msg::Gallery(items, st)) }));
        }
        Effect::LoadGallery(id) => spawn(Box::new(move || {
            let img = j.run(|env, act| load_photo(env, act, id)).flatten();
            Some(Msg::Photo(match img {
                Some(img) => { let img = crate::ocr::prepare(image::DynamicImage::ImageRgb8(img)); PHOTO_SRC_W.store(img.width(), std::sync::atomic::Ordering::Relaxed); App::photo_result(img) }
                None => PhotoState::Failed(crate::i18n::t("Could not open the photo").into()),
            }))
        })),
    }
}

fn density(a: &AndroidApp) -> f32 { a.config().density().map(|dpi| dpi as f32 / 160.0).unwrap_or(2.0) }

#[no_mangle]
fn android_main(a: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_tag("calculator"));
    let data = a.internal_data_path().unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp"));
    // crash catcher: a Rust panic writes its message to files/crash.txt before the app closes
    { let d = data.clone(); std::panic::set_hook(Box::new(move |info| {
        let msg = format!("Rust panic ({}): {}\n\n", crate::app::version(), info);
        log::error!("{msg}");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(d.join("crash.txt")) { use std::io::Write; let _ = f.write_all(msg.as_bytes()); }
    })); }
    // safe mode: if the last start never drew its first screen, skip the optional start-up extras this time
    let safe_mode = data.join("starting").exists();
    let _ = std::fs::write(data.join("starting"), b"1");
    let crash_text = std::fs::read_to_string(data.join("crash.txt")).unwrap_or_default();
    if !crash_text.is_empty() { let _ = std::fs::rename(data.join("crash.txt"), data.join("crash-last.txt")); }
    let saved: Saved = std::fs::read_to_string(data.join("saved.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut app = App::new(saved);
    if !crash_text.trim().is_empty() { app.crash = Some(crash_text.trim().to_string()); }
    let fonts = Fonts::new();
    let j = Jvm { vm: a.vm_as_ptr() as usize, act: a.activity_as_ptr() as usize };
    let waker = a.create_waker();
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || waker.wake());
    let updater = Updater::new(a.vm_as_ptr(), a.activity_as_ptr(), wake.clone());
    updater.check();
    let mut first_frame_done = false;
    let mut dirty_after = false;
    let (tx, rx) = channel::<Msg>();
    let mut window: Option<NativeWindow> = None;
    let (mut quit, mut dirty, mut camera_pending) = (false, true, false);
    let mut frame = app::Frame { widgets: vec![], scroll_area: None, page: (0, 0) };
    let mut pressed: Option<usize> = None;
    let mut down: Option<(f32, f32, Instant, bool)> = None; // x, y, time, scrolling
    let mut last_y = 0.0f32;
    let mut long_fired = false;
    let mut toast_at: Option<Instant> = None;
    let mut resumed = false;
    if app.s.conv_cat == crate::convert::CURRENCY_CAT || app.s.rates.is_none() { let _ = 0; }

    while !quit {
        let timeout = if down.is_some() || toast_at.is_some() { Some(Duration::from_millis(60)) } else { None };
        a.poll_events(timeout, |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => { window = a.native_window(); dirty = true; }
            PollEvent::Main(MainEvent::TerminateWindow { .. }) => { window = None; }
            PollEvent::Main(MainEvent::Resume { .. }) => { resumed = true; dirty = true; }
            PollEvent::Main(MainEvent::WindowResized { .. }) | PollEvent::Main(MainEvent::RedrawNeeded { .. }) | PollEvent::Main(MainEvent::ContentRectChanged { .. })
            | PollEvent::Main(MainEvent::ConfigChanged { .. }) | PollEvent::Main(MainEvent::GainedFocus) => { dirty = true; }
            PollEvent::Main(MainEvent::Destroy) => { quit = true; }
            _ => {}
        });
        let d = density(&a);
        let night = matches!(a.config().ui_mode_night(), ndk::configuration::UiModeNight::Yes);
        if night != app.system_dark { app.system_dark = night; dirty = true; }
        let sl = crate::i18n::lang_from_code(&a.config().language().unwrap_or_default());
        if sl != app.system_lang { app.system_lang = sl; app.apply_lang(); dirty = true; }
        let mut fx: Vec<Effect> = vec![];
        if resumed {
            resumed = false;
            if VOICE_PENDING.swap(false, std::sync::atomic::Ordering::Relaxed) { if let Some(t) = crate::addon::call(j, "voiceResult", "") { if !t.is_empty() { app.ask_text = t; app.ask_out = None; fx.extend(app.handle(&Action::Set("ask_solve"))); } } }
            if camera_pending { camera_pending = false; fx.extend(app.handle(&Action::OpenGallery)); }
            else if app.screen == Screen::Gallery && app.gallery.is_empty() { fx.push(Effect::OpenGallery); }
        }
        while let Ok(m) = rx.try_recv() {
            dirty = true;
            match m {
                Msg::RateHist(f, t, pts) => { app.toast = None; app.rate_hist = Some((f, t, pts)); }
                Msg::Rates(r) => match r { Some(r) => { app.rates_status = String::new(); app.s.rates = Some(r); fx.push(Effect::Save); } None => app.rates_status = crate::i18n::t("Could not update rates (showing last known)").into() },
                Msg::Markets(kind, quotes) => {
                    app.market_kind = kind;
                    if quotes.is_empty() {
                        app.market_status = "No market data received. Showing no stale values.".into();
                    } else {
                        app.market_quotes = quotes;
                        app.market_status = "Updated. Data may be delayed depending on the provider.".into();
                    }
                }
                Msg::Gallery(items, st) => { app.gallery = items; app.gallery_status = st; }
                Msg::Photo(p) => app.photo = p,
                Msg::Paste(t) => app.paste(&t),
                Msg::Toast(t) => app.toast = Some(t),
                Msg::Ask(r) => app.ask_result(r),
                Msg::Nano(n) => app.nano = n,
            }
        }
        let banner = match &*updater.state.lock().unwrap() {
            UpdateState::Idle => None,
            UpdateState::Available { tag, .. } => Some(crate::i18n::tf("Update {} - tap to install", &[tag])),
            UpdateState::Downloading { tag } => Some(crate::i18n::tf("Downloading {}…", &[tag])),
            UpdateState::Installing { tag } => Some(crate::i18n::tf("Confirm installing {}", &[tag])),
            UpdateState::Failed { message } => Some(crate::i18n::tf("{} (tap)", &[message])),
        };
        if banner != app.banner { app.banner = banner; dirty = true; }
        if app.toast.is_some() && toast_at.is_none() { toast_at = Some(Instant::now()); }
        if let Some(t) = toast_at { if t.elapsed() > Duration::from_millis(2200) { app.toast = None; toast_at = None; dirty = true; } }

        if let Ok(mut iter) = a.input_events_iter() {
            while iter.next(|ev| match ev {
                InputEvent::MotionEvent(m) => {
                    let p = m.pointer_at_index(m.pointer_index());
                    let (x, y) = (p.x(), p.y());
                    match m.action() {
                        MotionAction::Down => { pressed = frame.hit(x, y); down = Some((x, y, Instant::now(), false)); last_y = y; long_fired = false; dirty = true; }
                        MotionAction::Move => if let Some((x0, y0, t, scrolling)) = down {
                            let moved = ((x - x0).powi(2) + (y - y0).powi(2)).sqrt() > 10.0 * d;
                            let in_scroll = frame.scroll_area.map_or(false, |r| r.contains(x0, y0)) && ((y - y0).abs() > (x - x0).abs() || scrolling);
                            if moved && in_scroll { down = Some((x0, y0, t, true)); pressed = None; }
                            else if moved && pressed.is_some() && frame.hit(x, y) != pressed { pressed = None; }
                            if scrolling || (moved && in_scroll) {
                                let area_h = frame.scroll_area.map_or(0.0, |r| r.h);
                                app.scroll = (app.scroll - (y - last_y)).clamp(0.0, (app.content_h - area_h).max(0.0));
                            }
                            last_y = y; dirty = true;
                        },
                        MotionAction::Up => {
                            let swiped = if let Some((x0, y0, _, scrolling)) = down {
                                let (dx, dy) = (x - x0, y - y0);
                                if !scrolling && !long_fired && dx.abs() > 70.0 * d && dx.abs() > 2.0 * dy.abs() {
                                    let on = frame.hit(x0, y0).and_then(|i| frame.action(i)).unwrap_or(Action::None);
                                    let ok = match app.screen { Screen::Calc => matches!(on, Action::Copy | Action::ClosePopup), Screen::History => matches!(on, Action::LoadHistory(_)), _ => false };
                                    if ok { fx.extend(app.handle(&Action::Swipe(dx > 0.0, Box::new(on)))); }
                                    ok
                                } else { false }
                            } else { false };
                            if !long_fired && !swiped { if let (Some(i), Some(jj)) = (pressed, frame.hit(x, y)) { if i == jj { if let Some(act) = frame.action(i) { fx.extend(app.handle(&act)); } } } }
                            pressed = None; down = None; dirty = true;
                        }
                        MotionAction::Cancel => { pressed = None; down = None; dirty = true; }
                        _ => {}
                    }
                    InputStatus::Handled
                }
                InputEvent::KeyEvent(k) if k.key_code() == Keycode::Back && (app.screen != Screen::Calc || app.popup || app.text_target.is_some()) => {
                    if k.action() == KeyAction::Up { fx.extend(app.handle(if app.text_target.is_some() { &Action::EndText } else { &Action::Back })); dirty = true; }
                    InputStatus::Handled
                }
                InputEvent::KeyEvent(k) if k.action() == KeyAction::Down && k.key_code() != Keycode::Back && !matches!(k.key_code(), Keycode::VolumeUp | Keycode::VolumeDown) => {
                    let ch = match k.key_code() {
                        Keycode::Del => Some('\u{8}'), Keycode::Enter | Keycode::NumpadEnter => Some('\n'), Keycode::Escape => Some('\u{1b}'),
                        code => a.device_key_character_map(k.device_id()).ok().and_then(|m| m.get(code, k.meta_state()).ok()).and_then(|c| match c { android_activity::input::KeyMapChar::Unicode(c) => Some(c), _ => None }),
                    };
                    match (ch, app.text_target.is_some()) {
                        (Some('\u{8}'), true) => fx.extend(app.handle(&Action::TextBack)),
                        (Some('\n'), true) => fx.extend(app.handle(&Action::EndText)),
                        (Some(c), true) if !c.is_control() => fx.extend(app.handle(&Action::TextKey(c))),
                        (Some(c), false) if app.screen == Screen::Calc => fx.extend(app.handle(&Action::HwKey(c))),
                        _ => return InputStatus::Unhandled,
                    }
                    dirty = true;
                    InputStatus::Handled
                }
                _ => InputStatus::Unhandled,
            }) {}
        }
        // long press on the display: copy / paste menu
        if let (Some((_, _, t, false)), Some(i)) = (down, pressed) {
            if !long_fired && app.screen == Screen::Calc && t.elapsed() > Duration::from_millis(450) && frame.action(i) == Some(Action::ClosePopup) {
                long_fired = true; pressed = None; fx.extend(app.handle(&Action::DisplayLongPress)); dirty = true;
            }
        }
        for e in fx {
            if let Effect::Keyboard(show) = e { if show { a.show_soft_input(true); } else { a.hide_soft_input(false); } continue; }
            effect(e, &mut app, j, &tx, &wake, &updater, &data, &mut camera_pending); dirty = true; }

        if dirty {
            if let Some(win) = window.as_ref() {
                let (w, h) = (win.width(), win.height());
                if w > 0 && h > 0 {
                    let _ = win.set_buffers_geometry(0, 0, Some(HardwareBufferFormat::R8G8B8A8_UNORM));
                    if let Ok(mut g) = win.lock(None) {
                        let (bw, bh, stride) = (g.width(), g.height(), g.stride());
                        frame = app.build(bw as f32, bh as f32, d);
                        let px = unsafe { std::slice::from_raw_parts_mut(g.bits() as *mut u8, stride * bh * 4) };
                        let mut cv = Canvas { px, w: bw, h: bh, stride };
                        app::draw(&app, &frame, &fonts, &mut cv, d, pressed);
                        if !first_frame_done {
                            first_frame_done = true;
                            let _ = std::fs::remove_file(data.join("starting"));
                            // optional extras from the Java add-on, only after the app is on screen
                            if !safe_mode {
                                if let Some(h) = crate::addon::call(j, "accent", "") { if let Ok(v) = u32::from_str_radix(h.trim_start_matches('#'), 16) { app.system_accent = Some([(v >> 16) as u8, (v >> 8) as u8, v as u8]); } }
                                if let Some(f) = crate::addon::call(j, "fontScale", "").and_then(|f| f.parse::<f32>().ok()) { app.system_font = f; }
                                app.apply_look();
                                let _ = crate::addon::call(j, "updateChecks", if app.s.bg_check { "1" } else { "0" });
                            }
                            // the phone's own record of an earlier crash (Android 11+)
                            if let Some(t) = crate::addon::call(j, "lastCrash", "") { if !t.trim().is_empty() && app.crash.as_deref().map_or(true, |c| !c.contains(t.trim())) {
                                app.crash = Some(match app.crash.take() { Some(c) => format!("{c}\n\n{}", t.trim()), None => t.trim().to_string() }); } }
                            dirty_after = true;
                        }
                    }
                }
            }
            dirty = false;
            if dirty_after { dirty_after = false; dirty = true; }
        }
    }
}

fn enc(s: &str) -> String { s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{:02X}", b) }).collect() }

static VOICE_PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Used by the Java add-on (floating calculator, widget): evaluates an expression with the app's engine.
#[no_mangle]
pub extern "system" fn Java_com_example_androidcalculator_rust_Addon_nativeEval<'a>(mut env: JNIEnv<'a>, _c: jni::objects::JClass<'a>, s: jni::objects::JString<'a>) -> jni::sys::jstring {
    let text: String = env.get_string(&s).map(|v| v.into()).unwrap_or_default();
    let out = crate::expr::eval_text(&text).map(crate::expr::display_num).unwrap_or_else(|| "Error".into());
    env.new_string(out).map(|o| o.into_raw()).unwrap_or(std::ptr::null_mut())
}
