// Simulates the start of android_main with a given saved.json (or none) and draws the first frames.
use calculator::app::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let saved: Saved = a.get(1).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).map_err(|e| eprintln!("parse err: {e}")).ok()).unwrap_or_default();
    let mut app = App::new(saved);
    let fonts = calculator::draw::Fonts::new();
    app.system_accent = Some([0x44, 0x66, 0x88]); app.system_font = std::env::var("FS").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0); app.apply_look();
    for (lang, dark) in [("lt", false), ("lt", true), ("en", false), ("ru", false), ("pl", false), ("de", false), ("uk", false), ("lv", false), ("fr", false), ("es", false), ("zh", true), ("", false)] {
        app.system_dark = dark; app.system_lang = calculator::i18n::lang_from_code(lang); app.apply_lang();
        for (w, h, d) in [(720u32, 1600u32, 1.5f32), (720, 1600, 1.75), (720, 1600, 1.875), (720, 1600, 2.0), (720, 1600, 2.25), (1600, 720, 2.0), (720, 1500, 2.0), (1, 1, 2.0), (720, 100, 2.0)] {
            let frame = app.build(w as f32, h as f32, d);
            let mut px = vec![0u8; (w * h * 4) as usize];
            let mut cv = calculator::draw::Canvas { px: &mut px, w: w as usize, h: h as usize, stride: w as usize };
            draw(&app, &frame, &fonts, &mut cv, d, None);
        }
    }
    println!("ok whats_new={} screen={:?} lang={}", app.whats_new, app.screen, app.s.lang);
}
