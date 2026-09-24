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
