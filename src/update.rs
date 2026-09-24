//! Auto-update: checks the latest GitHub release and, if it is newer than this build,
//! downloads its .apk with Android's DownloadManager and opens the system installer.
//! Version logic and release parsing are plain Rust (unit-tested on the host);
//! Android calls go through JNI to framework classes only (no Java code in the app).

use serde::Deserialize;

/// GitHub repo checked for releases. Override at build time: UPDATE_REPO=owner/name ./build_apk.sh
/// It must be PUBLIC: the app has no GitHub token, and private repos return 404 to anonymous requests.
pub const UPDATE_REPO: &str = match option_env!("UPDATE_REPO") { Some(r) => r, None => "Senelis123/android-calculator" };


#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    Available { tag: String, apk_url: String },
    Downloading { tag: String },
    Installing { tag: String },
    Failed { message: String },
}

#[derive(Deserialize)]
struct Release { tag_name: String, #[serde(default)] draft: bool, #[serde(default)] prerelease: bool, #[serde(default)] assets: Vec<Asset> }
#[derive(Deserialize)]
struct Asset { name: String, browser_download_url: String }

/// "v2.10.1" -> [2, 10, 1]; ignores a leading "v" and any "-suffix".
pub fn parse_version(s: &str) -> Vec<u64> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+', ' ']).next().unwrap_or("");
    core.split('.').map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect()
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (mut a, mut b) = (parse_version(candidate), parse_version(current));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

/// Parses a `GET /repos/{repo}/releases/latest` body. Returns (tag, apk url) if it is a newer release with an .apk asset.
pub fn newer_release(json: &str, current: &str) -> Option<(String, String)> {
    let r: Release = serde_json::from_str(json).ok()?;
    if r.draft || r.prerelease || !is_newer(&r.tag_name, current) { return None; }
    let apk = r.assets.iter().find(|a| a.name.to_ascii_lowercase().ends_with(".apk"))?;
    Some((r.tag_name, apk.browser_download_url.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn versions() {
        assert!(is_newer("v2.2.0", "2.1.0"));
        assert!(is_newer("2.10", "2.9.9"));
        assert!(is_newer("v3", "2.1.0"));
        assert!(!is_newer("v2.1.0", "2.1.0"));
        assert!(!is_newer("v2.1", "2.1.0"));
        assert!(!is_newer("v2.0.9", "2.1.0"));
        assert!(is_newer("v2.1.1-beta", "2.1.0"));
    }
    #[test] fn release_parsing() {
        let j = r#"{"tag_name":"v2.2.0","draft":false,"prerelease":false,"assets":[
            {"name":"notes.txt","browser_download_url":"https://x/notes.txt"},
            {"name":"calculator-rust.apk","browser_download_url":"https://github.com/o/r/releases/download/v2.2.0/calculator-rust.apk"}]}"#;
        assert_eq!(newer_release(j, "2.1.0"), Some(("v2.2.0".into(), "https://github.com/o/r/releases/download/v2.2.0/calculator-rust.apk".into())));
        assert_eq!(newer_release(j, "2.2.0"), None);
        assert_eq!(newer_release(r#"{"tag_name":"v9","assets":[]}"#, "2.1.0"), None);
        assert_eq!(newer_release(r#"{"message":"Not Found"}"#, "2.1.0"), None);
        assert_eq!(newer_release(r#"{"tag_name":"v9","prerelease":true,"assets":[{"name":"a.apk","browser_download_url":"u"}]}"#, "2.1.0"), None);
    }
}

#[cfg(target_os = "android")]
pub mod android {
    use super::*;
    use jni::objects::{JObject, JValue};
    use jni::JNIEnv;
    use std::sync::{Arc, Mutex};

    pub(crate) type R<T> = jni::errors::Result<T>;

    pub(crate) fn clear(env: &mut JNIEnv) { if env.exception_check().unwrap_or(false) { let _ = env.exception_describe(); let _ = env.exception_clear(); } }

    pub(crate) fn http_get(env: &mut JNIEnv, url: &str) -> R<Option<String>> {
        let jurl = env.new_string(url)?;
        let u = env.new_object("java/net/URL", "(Ljava/lang/String;)V", &[JValue::Object(&jurl)])?;
        let conn = env.call_method(&u, "openConnection", "()Ljava/net/URLConnection;", &[])?.l()?;
        for (k, v) in [("Accept", "application/vnd.github+json"), ("User-Agent", "calculator-rust-updater")] {
            let (k, v) = (env.new_string(k)?, env.new_string(v)?);
            env.call_method(&conn, "setRequestProperty", "(Ljava/lang/String;Ljava/lang/String;)V", &[JValue::Object(&k), JValue::Object(&v)])?;
        }
        env.call_method(&conn, "setConnectTimeout", "(I)V", &[JValue::Int(10_000)])?;
        env.call_method(&conn, "setReadTimeout", "(I)V", &[JValue::Int(10_000)])?;
        let code = env.call_method(&conn, "getResponseCode", "()I", &[])?.i()?;
        if code != 200 { log::info!("update check: HTTP {code} for {url}"); return Ok(None); }
        let input = env.call_method(&conn, "getInputStream", "()Ljava/io/InputStream;", &[])?.l()?;
        let cs = env.new_string("UTF-8")?;
        let scanner = env.new_object("java/util/Scanner", "(Ljava/io/InputStream;Ljava/lang/String;)V", &[JValue::Object(&input), JValue::Object(&cs)])?;
        let delim = env.new_string("\\A")?;
        let scanner = env.call_method(&scanner, "useDelimiter", "(Ljava/lang/String;)Ljava/util/Scanner;", &[JValue::Object(&delim)])?.l()?;
        let body = env.call_method(&scanner, "next", "()Ljava/lang/String;", &[])?.l()?;
        let body: String = env.get_string(&body.into())?.into();
        env.call_method(&conn, "disconnect", "()V", &[])?;
        Ok(Some(body))
    }

    /// Returns the download id.
    fn enqueue_download(env: &mut JNIEnv, activity: &JObject, url: &str, tag: &str) -> R<i64> {
        let svc = env.new_string("download")?;
        let dm = env.call_method(activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&svc)])?.l()?;
        let jurl = env.new_string(url)?;
        let uri = env.call_static_method("android/net/Uri", "parse", "(Ljava/lang/String;)Landroid/net/Uri;", &[JValue::Object(&jurl)])?.l()?;
        let req = env.new_object("android/app/DownloadManager$Request", "(Landroid/net/Uri;)V", &[JValue::Object(&uri)])?;
        let mime = env.new_string("application/vnd.android.package-archive")?;
        env.call_method(&req, "setMimeType", "(Ljava/lang/String;)Landroid/app/DownloadManager$Request;", &[JValue::Object(&mime)])?;
        let title = env.new_string(format!("Calculator {tag}"))?;
        env.call_method(&req, "setTitle", "(Ljava/lang/CharSequence;)Landroid/app/DownloadManager$Request;", &[JValue::Object(&title)])?;
        env.call_method(&req, "setNotificationVisibility", "(I)Landroid/app/DownloadManager$Request;", &[JValue::Int(0)])?; // VISIBILITY_VISIBLE
        let dir = env.new_string("Download")?;
        let name = env.new_string(format!("calculator-{tag}-{}.apk", std::process::id()))?;
        env.call_method(&req, "setDestinationInExternalFilesDir", "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;)Landroid/app/DownloadManager$Request;",
            &[JValue::Object(activity), JValue::Object(&dir), JValue::Object(&name)])?;
        env.call_method(&dm, "enqueue", "(Landroid/app/DownloadManager$Request;)J", &[JValue::Object(&req)])?.j()
    }

    /// 8 = successful, 16 = failed, other = pending/running.
    fn download_status(env: &mut JNIEnv, activity: &JObject, id: i64) -> R<i32> {
        let svc = env.new_string("download")?;
        let dm = env.call_method(activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&svc)])?.l()?;
        let q = env.new_object("android/app/DownloadManager$Query", "()V", &[])?;
        let ids = env.new_long_array(1)?;
        env.set_long_array_region(&ids, 0, &[id])?;
        env.call_method(&q, "setFilterById", "([J)Landroid/app/DownloadManager$Query;", &[JValue::Object(&ids)])?;
        let cursor = env.call_method(&dm, "query", "(Landroid/app/DownloadManager$Query;)Landroid/database/Cursor;", &[JValue::Object(&q)])?.l()?;
        if cursor.is_null() { return Ok(16); }
        let mut status = 16;
        if env.call_method(&cursor, "moveToFirst", "()Z", &[])?.z()? {
            let col = env.new_string("status")?;
            let idx = env.call_method(&cursor, "getColumnIndex", "(Ljava/lang/String;)I", &[JValue::Object(&col)])?.i()?;
            status = env.call_method(&cursor, "getInt", "(I)I", &[JValue::Int(idx)])?.i()?;
        }
        env.call_method(&cursor, "close", "()V", &[])?;
        Ok(status)
    }

    fn launch_installer(env: &mut JNIEnv, activity: &JObject, id: i64) -> R<()> {
        let svc = env.new_string("download")?;
        let dm = env.call_method(activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[JValue::Object(&svc)])?.l()?;
        let uri = env.call_method(&dm, "getUriForDownloadedFile", "(J)Landroid/net/Uri;", &[JValue::Long(id)])?.l()?;
        let action = env.new_string("android.intent.action.VIEW")?;
        let intent = env.new_object("android/content/Intent", "(Ljava/lang/String;)V", &[JValue::Object(&action)])?;
        let mime = env.new_string("application/vnd.android.package-archive")?;
        env.call_method(&intent, "setDataAndType", "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;", &[JValue::Object(&uri), JValue::Object(&mime)])?;
        // FLAG_GRANT_READ_URI_PERMISSION | FLAG_ACTIVITY_NEW_TASK
        env.call_method(&intent, "addFlags", "(I)Landroid/content/Intent;", &[JValue::Int(0x0000_0001 | 0x1000_0000)])?;
        env.call_method(activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::Object(&intent)])?;
        Ok(())
    }

    pub struct Updater { pub state: Arc<Mutex<UpdateState>>, vm: usize, activity: usize, wake: Arc<dyn Fn() + Send + Sync> }

    impl Updater {
        pub fn new(vm: *mut std::ffi::c_void, activity: *mut std::ffi::c_void, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
            Updater { state: Arc::new(Mutex::new(UpdateState::Idle)), vm: vm as usize, activity: activity as usize, wake }
        }

        fn spawn<F: FnOnce(&mut JNIEnv, &JObject, &Arc<Mutex<UpdateState>>) + Send + 'static>(&self, f: F) {
            let (vm, act, state, wake) = (self.vm, self.activity, self.state.clone(), self.wake.clone());
            std::thread::spawn(move || {
                let vm = match unsafe { jni::JavaVM::from_raw(vm as *mut _) } { Ok(v) => v, Err(_) => return };
                let mut env = match vm.attach_current_thread() { Ok(e) => e, Err(_) => return };
                let activity = unsafe { JObject::from_raw(act as jni::sys::jobject) };
                f(&mut env, &activity, &state);
                clear(&mut env);
                wake();
            });
        }

        /// Background check of the latest release; silent on any failure.
        pub fn check(&self) {
            self.spawn(|env, _act, state| {
                let url = format!("https://api.github.com/repos/{UPDATE_REPO}/releases/latest");
                match http_get(env, &url) {
                    Ok(Some(body)) => {
                        if let Some((tag, apk_url)) = newer_release(&body, crate::app::version()) {
                            *state.lock().unwrap() = UpdateState::Available { tag, apk_url };
                        }
                    }
                    Ok(None) => {}
                    Err(e) => { log::warn!("update check failed: {e:?}"); clear(env); }
                }
            });
        }

        /// User tapped the update banner: download, then open the installer.
        pub fn start(&self) {
            let (tag, url) = match &*self.state.lock().unwrap() {
                UpdateState::Available { tag, apk_url } => (tag.clone(), apk_url.clone()),
                _ => return,
            };
            *self.state.lock().unwrap() = UpdateState::Downloading { tag: tag.clone() };
            let wake = self.wake.clone();
            self.spawn(move |env, act, state| {
                let fail = |env: &mut JNIEnv, msg: &str| { clear(env); *state.lock().unwrap() = UpdateState::Failed { message: msg.into() }; };
                wake();
                let id = match enqueue_download(env, act, &url, &tag) { Ok(id) => id, Err(e) => { log::warn!("{e:?}"); return fail(env, "Nepavyko pradėti atsisiuntimo"); } };
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    match download_status(env, act, id) {
                        Ok(8) => break,
                        Ok(16) => return fail(env, "Atsisiųsti nepavyko"),
                        Ok(_) => continue,
                        Err(e) => { log::warn!("{e:?}"); return fail(env, "Atsisiųsti nepavyko"); }
                    }
                }
                match launch_installer(env, act, id) {
                    Ok(()) => *state.lock().unwrap() = UpdateState::Installing { tag: tag.clone() },
                    Err(e) => { log::warn!("{e:?}"); fail(env, "Nepavyko atidaryti diegimo"); }
                }
            });
        }
    }
}
