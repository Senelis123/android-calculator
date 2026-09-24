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

enum Msg { Rates(Option<crate::convert::Rates>), Gallery(Vec<GalleryItem>, String), Photo(PhotoState), Paste(String), Toast(String) }

#[derive(Clone, Copy)]
struct Jvm { vm: usize, act: usize }
impl Jvm {
    fn run<T>(&self, f: impl FnOnce(&mut JNIEnv, &JObject) -> R<T>) -> Option<T> {
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
        Effect::Vibrate => { j.run(|env, act| { let v = service(env, act, "vibrator")?; if !v.is_null() { env.call_method(&v, "vibrate", "(J)V", &[JValue::Long(12)])?; } Ok(()) }); }
        Effect::Copy(text) => { j.run(|env, act| {
            ensure_looper(env)?;
            let cm = service(env, act, "clipboard")?;
            let (l, t) = (env.new_string("Calculator")?, env.new_string(&text)?);
            let clip = env.call_static_method("android/content/ClipData", "newPlainText", "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;", &[JValue::Object(&l), JValue::Object(&t)])?.l()?;
            env.call_method(&cm, "setPrimaryClip", "(Landroid/content/ClipData;)V", &[JValue::Object(&clip)])?; Ok(())
        }); }
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
            match t { Some(t) => { let _ = tx.send(Msg::Paste(t)); } None => { let _ = tx.send(Msg::Toast("Iškarpinė tuščia".into())); } }
        }
        Effect::Save => { if let Ok(js) = serde_json::to_string(&app.s) { let tmp = data.join("saved.json.tmp"); if std::fs::write(&tmp, js).is_ok() { let _ = std::fs::rename(&tmp, data.join("saved.json")); } } }
        Effect::FetchRates => { app.rates_status = "Atnaujinami kursai…".into(); spawn(Box::new(move || {
            let body = j.run(|env, _| http_get(env, "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml")).flatten();
            Some(Msg::Rates(body.and_then(|b| crate::convert::parse_ecb(&b))))
        })); }
        Effect::CheckUpdate => updater.check(),
        Effect::UpdateTap => {
            let st = updater.state.lock().unwrap().clone();
            match st { UpdateState::Available { .. } => updater.start(), UpdateState::Failed { .. } | UpdateState::Installing { .. } => *updater.state.lock().unwrap() = UpdateState::Idle, _ => {} }
        }
        Effect::TakePhoto => {
            let ok = j.run(|env, act| { let a = env.new_string("android.media.action.STILL_IMAGE_CAMERA")?; let i = env.new_object("android/content/Intent", "(Ljava/lang/String;)V", &[JValue::Object(&a)])?;
                env.call_method(act, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&i)])?; Ok(()) }).is_some();
            if ok { *camera_pending = true; app.toast = Some("Nufotografuokite ir grįžkite - nuotrauka bus sąraše".into()); } else { app.toast = Some("Nepavyko atidaryti kameros".into()); }
        }
        Effect::OpenGallery => {
            if j.run(|env, act| photo_permission(env, act, true)) != Some(true) { app.gallery_status = "Leiskite programai matyti nuotraukas ir bandykite dar kartą.".into(); return; }
            app.gallery_status = "Įkeliama…".into();
            spawn(Box::new(move || { let items = j.run(|env, act| list_gallery(env, act)).unwrap_or_default();
                let st = if items.is_empty() { "Nuotraukų nerasta".to_string() } else { String::new() }; Some(Msg::Gallery(items, st)) }));
        }
        Effect::LoadGallery(id) => spawn(Box::new(move || {
            let img = j.run(|env, act| load_photo(env, act, id)).flatten();
            Some(Msg::Photo(match img {
                Some(img) => { let img = crate::ocr::prepare(image::DynamicImage::ImageRgb8(img)); PHOTO_SRC_W.store(img.width(), std::sync::atomic::Ordering::Relaxed); App::photo_result(img) }
                None => PhotoState::Failed("Nepavyko atidaryti nuotraukos".into()),
            }))
        })),
    }
}

fn density(a: &AndroidApp) -> f32 { a.config().density().map(|dpi| dpi as f32 / 160.0).unwrap_or(2.0) }

#[no_mangle]
fn android_main(a: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_tag("calculator"));
    let data = a.internal_data_path().unwrap_or_else(|| std::path::PathBuf::from("/data/local/tmp"));
    let saved: Saved = std::fs::read_to_string(data.join("saved.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let mut app = App::new(saved);
    let fonts = Fonts::new();
    let j = Jvm { vm: a.vm_as_ptr() as usize, act: a.activity_as_ptr() as usize };
    let waker = a.create_waker();
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || waker.wake());
    let updater = Updater::new(a.vm_as_ptr(), a.activity_as_ptr(), wake.clone());
    updater.check();
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
        let mut fx: Vec<Effect> = vec![];
        if resumed {
            resumed = false;
            if camera_pending { camera_pending = false; fx.extend(app.handle(&Action::OpenGallery)); }
            else if app.screen == Screen::Gallery && app.gallery.is_empty() { fx.push(Effect::OpenGallery); }
        }
        while let Ok(m) = rx.try_recv() {
            dirty = true;
            match m {
                Msg::Rates(r) => match r { Some(r) => { app.rates_status = String::new(); app.s.rates = Some(r); fx.push(Effect::Save); } None => app.rates_status = "Nepavyko atnaujinti kursų (rodomi paskutiniai žinomi)".into() },
                Msg::Gallery(items, st) => { app.gallery = items; app.gallery_status = st; }
                Msg::Photo(p) => app.photo = p,
                Msg::Paste(t) => app.paste(&t),
                Msg::Toast(t) => app.toast = Some(t),
            }
        }
        let banner = match &*updater.state.lock().unwrap() {
            UpdateState::Idle => None,
            UpdateState::Available { tag, .. } => Some(format!("Naujinys {tag} - bakstelėkite įdiegti")),
            UpdateState::Downloading { tag } => Some(format!("Atsisiunčiamas {tag}…")),
            UpdateState::Installing { tag } => Some(format!("Patvirtinkite {tag} diegimą")),
            UpdateState::Failed { message } => Some(format!("{message} (bakstelėkite)")),
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
                            let in_scroll = frame.scroll_area.map_or(false, |r| r.contains(x0, y0));
                            if moved && in_scroll { down = Some((x0, y0, t, true)); pressed = None; }
                            else if moved && pressed.is_some() && frame.hit(x, y) != pressed { pressed = None; }
                            if scrolling || (moved && in_scroll) {
                                let area_h = frame.scroll_area.map_or(0.0, |r| r.h);
                                app.scroll = (app.scroll - (y - last_y)).clamp(0.0, (app.content_h - area_h).max(0.0));
                            }
                            last_y = y; dirty = true;
                        },
                        MotionAction::Up => {
                            if !long_fired { if let (Some(i), Some(jj)) = (pressed, frame.hit(x, y)) { if i == jj { if let Some(act) = frame.action(i) { fx.extend(app.handle(&act)); } } } }
                            pressed = None; down = None; dirty = true;
                        }
                        MotionAction::Cancel => { pressed = None; down = None; dirty = true; }
                        _ => {}
                    }
                    InputStatus::Handled
                }
                InputEvent::KeyEvent(k) if k.key_code() == Keycode::Back && (app.screen != Screen::Calc || app.popup) => {
                    if k.action() == KeyAction::Up { fx.extend(app.handle(&Action::Back)); dirty = true; }
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
        for e in fx { effect(e, &mut app, j, &tx, &wake, &updater, &data, &mut camera_pending); dirty = true; }

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
                    }
                }
            }
            dirty = false;
        }
    }
}
