// Renders app screens to PNG for visual checks. Usage: screens OUT.png WxH [dark] actions...
// actions: key:<k> nav:<screen> sci theme:<n> photo:<file>
use calculator::app::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (w, h) = { let p: Vec<u32> = a[2].split('x').map(|v| v.parse().unwrap()).collect(); (p[0], p[1]) };
    let d = 2.625f32;
    let mut app = App::default();
    for s in &a[3..] {
        if let Some(k) = s.strip_prefix("key:") { app.handle(&Action::Key(k.into())); }
        else if s == "sci" { app.handle(&Action::ToggleSci); }
        else if s == "dark" { app.s.theme = 2; }
        else if s == "longpress" { app.handle(&Action::DisplayLongPress); }
        else if let Some(t) = s.strip_prefix("toast:") { app.toast = Some(t.into()); }
        else if let Some(t) = s.strip_prefix("banner:") { app.banner = Some(t.into()); }
        else if let Some(f) = s.strip_prefix("photo:") {
            let img = calculator::ocr::prepare(image::open(f).unwrap());
            PHOTO_SRC_W.store(img.width(), std::sync::atomic::Ordering::Relaxed);
            app.photo = App::photo_result(img); app.screen = Screen::Photo;
        }
        else if let Some(n) = s.strip_prefix("nav:") {
            let sc = match n { "history" => Screen::History, "vat" => Screen::Vat, "conv" => Screen::Converter, "settings" => Screen::Settings, "photo" => Screen::Photo, "pick" => Screen::PickUnit { from: true }, _ => Screen::Calc };
            app.handle(&Action::Nav(sc));
        }
        else if let Some(n) = s.strip_prefix("tool:") { app.handle(&Action::OpenTool(n.parse().unwrap())); }
        else if let Some(n) = s.strip_prefix("mode:") { app.handle(&Action::ToolMode(n.parse().unwrap())); }
        else if let Some(n) = s.strip_prefix("focus:") { app.handle(&Action::Focus(n.parse().unwrap())); }
        else if let Some(k) = s.strip_prefix("fk:") { for c in k.split('_') { let c: &'static str = Box::leak(c.to_string().into_boxed_str()); app.handle(&Action::FormKey(c)); } }
        else if s == "tools" { app.handle(&Action::Nav(Screen::Tools)); }
        else if let Some(t) = s.strip_prefix("ask:") { app.handle(&Action::Nav(Screen::Ask)); app.ask_text = t.replace('_', " "); app.nano = "unavailable".into(); let e = calculator::expr::text_to_expr(&app.ask_text).map(|e| (e, false)); app.ask_result(e); }
        else if let Some(n) = s.strip_prefix("scroll:") { app.scroll = n.parse().unwrap(); }
        else if s == "whatsnew" { app.whats_new = true; }
        else if s == "crash" { app.crash = Some("Java error (main, 2.3.0.0):\njava.lang.RuntimeException: Unable to start activity ComponentInfo{com.example.androidcalculator.rust/android.app.NativeActivity}: java.lang.IllegalArgumentException: Unable to find native library main using classloader: PathClassLoader[DexPathList[[zip file \"/data/app/~~abc/base.apk\"],nativeLibraryDirectories=[/data/app/~~abc/lib/arm64, /system/lib64]]]\n\tat android.app.NativeActivity.onCreate(NativeActivity.java:164)\n\tat android.app.Activity.performCreate(Activity.java:8960)\n\tat android.app.Activity.performCreate(Activity.java:8938)\n\tat android.app.Instrumentation.callActivityOnCreate(Instrumentation.java:1526)\n\tat android.app.ActivityThread.performLaunchActivity(ActivityThread.java:4262)\n\tat android.app.ActivityThread.handleLaunchActivity(ActivityThread.java:4467)\n\tat android.app.servertransaction.LaunchActivityItem.execute(LaunchActivityItem.java:222)\n\tat android.app.servertransaction.TransactionExecutor.executeNonLifecycleItem(TransactionExecutor.java:133)\n\tat android.app.servertransaction.TransactionExecutor.executeLifecycleItemSequence(TransactionExecutor.java:103)".repeat(3)); }
        else if s == "onehand" { app.s.one_hand = 1; }
        else if s == "graph" { app.handle(&Action::Nav(Screen::Graph)); }
        else if let Some(l) = s.strip_prefix("lang:") { app.s.lang = l.parse().unwrap(); app.apply_lang(); }
        else if let Some(c) = s.strip_prefix("cat:") { app.handle(&Action::ConvCat(c.parse().unwrap())); }
    }
    if app.s.rates.is_none() { app.s.rates = calculator::convert::parse_ecb("<Cube time='2026-09-24'><Cube currency='USD' rate='1.1367'/><Cube currency='GBP' rate='0.85986'/></Cube>"); }
    let fonts = calculator::draw::Fonts::new();
    let frame = app.build(w as f32, h as f32, d);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut cv = calculator::draw::Canvas { px: &mut px, w: w as usize, h: h as usize, stride: w as usize };
    draw(&app, &frame, &fonts, &mut cv, d, None);
    image::save_buffer(&a[1], &px, w, h, image::ExtendedColorType::Rgba8).unwrap();
}
