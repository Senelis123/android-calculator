//! Application state, key handling and screen layout (as a list of widgets).

use crate::convert::{self, Rates, CATEGORIES, CURRENCY_CAT};
use crate::draw::{Canvas, Fonts, Rect, Rgb};
use crate::expr::{self, display_num, display_tokens, plain, Evaluator, SolveError, Tok};
use serde::{Deserialize, Serialize};

pub const VERSION: &str = include_str!("../VERSION");
pub fn version() -> &'static str { VERSION.trim() }

// ---------------- persistent data ----------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistItem { pub expr: String, pub result: String, pub value: Option<f64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub theme: u8,       // 0 auto, 1 light, 2 dark
    pub vibrate: bool,
    pub degrees: bool,
    pub sci: bool,
    pub memory: f64,
    pub history: Vec<HistItem>,
    pub conv_cat: usize,
    pub conv_from: Vec<usize>,
    pub conv_to: Vec<usize>,
    pub rates: Option<Rates>,
    pub tip: usize,
    pub people: usize,
}

impl Default for Saved {
    fn default() -> Self {
        Saved { theme: 0, vibrate: true, degrees: true, sci: false, memory: 0.0, history: vec![], conv_cat: 1,
            conv_from: vec![0, 3, 2, 0, 1, 1, 1], conv_to: vec![1, 7, 5, 1, 3, 3, 2], rates: None, tip: 1, people: 1 }
    }
}

// ---------------- runtime state ----------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen { Calc, History, Vat, Converter, PickUnit { from: bool }, Settings, Photo, Gallery }

pub enum PhotoState {
    Idle,
    Working,
    Done { img: image::RgbImage, bbox: Option<[f32; 4]>, text: String, tokens: Vec<Tok> },
    Failed(String),
}

pub struct GalleryItem { pub id: i64, pub thumb: Option<image::RgbImage>, pub label: String }

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Key(String),
    Nav(Screen),
    Back,
    UseValue(f64),
    UseTokens,
    LoadHistory(usize),
    ClearHistory,
    ToggleSci,
    CycleTheme,
    ToggleVibrate,
    ToggleAngle,
    CheckUpdate,
    UpdateBanner,
    Copy,
    Paste,
    DisplayLongPress,
    ClosePopup,
    ConvCat(usize),
    PickUnit(usize),
    SwapUnits,
    RefreshRates,
    Tip(usize),
    People(usize),
    TakePhoto,
    OpenGallery,
    GalleryPick(i64),
    None,
}

/// Side effects the platform layer performs.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect { Vibrate, Copy(String), RequestPaste, Save, FetchRates, CheckUpdate, UpdateTap, TakePhoto, OpenGallery, LoadGallery(i64) }

pub struct App {
    pub s: Saved,
    pub tokens: Vec<Tok>,
    pub just_evaluated: bool,
    pub result: Option<(String, String)>, // (expression line, result text) after "="
    pub screen: Screen,
    pub scroll: f32,
    pub content_h: f32,
    pub popup: bool,
    pub banner: Option<String>,
    pub toast: Option<String>,
    pub system_dark: bool,
    pub photo: PhotoState,
    pub gallery: Vec<GalleryItem>,
    pub gallery_status: String,
    pub rates_status: String,
}

impl Default for App { fn default() -> Self { Self::new(Saved::default()) } }

fn is_value_end(t: Option<&Tok>) -> bool { matches!(t, Some(Tok::Num(_) | Tok::RParen | Tok::Pi | Tok::E | Tok::X | Tok::Post(_))) }

impl App {
    pub fn new(s: Saved) -> Self {
        App { s, tokens: vec![], just_evaluated: false, result: None, screen: Screen::Calc, scroll: 0.0, content_h: 0.0,
            popup: false, banner: None, toast: None, system_dark: false, photo: PhotoState::Idle, gallery: vec![],
            gallery_status: String::new(), rates_status: String::new() }
    }

    pub fn dark(&self) -> bool { match self.s.theme { 1 => false, 2 => true, _ => self.system_dark } }
    fn ev(&self) -> Evaluator { Evaluator { degrees: self.s.degrees } }

    /// The value the calculator currently shows (result, or live value of the expression).
    pub fn current_value(&self) -> Option<f64> {
        if self.tokens.iter().any(|t| matches!(t, Tok::X | Tok::Equals)) { return None; }
        self.ev().eval(&self.tokens).ok()
    }

    fn open_parens(&self) -> i32 {
        self.tokens.iter().map(|t| match t { Tok::LParen | Tok::Func(_) => 1, Tok::RParen => -1, _ => 0 }).sum()
    }

    fn start_fresh_if_evaluated(&mut self, keep_value: bool) {
        if self.just_evaluated {
            if !keep_value { self.tokens.clear(); }
            self.just_evaluated = false;
            self.result = None;
        }
    }

    pub fn set_value(&mut self, v: f64) {
        self.tokens = vec![Tok::Num(plain(v))];
        self.just_evaluated = true;
        self.result = None;
    }

    fn push_value_tok(&mut self, t: Tok) {
        self.start_fresh_if_evaluated(false);
        self.tokens.push(t);
    }

    pub fn press(&mut self, key: &str) {
        match key {
            "C" => { self.tokens.clear(); self.just_evaluated = false; self.result = None; }
            "⌫" => {
                if self.just_evaluated { return; }
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) => { n.pop(); if n.is_empty() || n == "-" { self.tokens.pop(); } }
                    Some(_) => { self.tokens.pop(); }
                    None => {}
                }
            }
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                self.start_fresh_if_evaluated(false);
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) if n.len() < 20 => { if n == "0" { *n = key.into(); } else if n == "-0" { *n = format!("-{key}"); } else { n.push_str(key); } }
                    Some(Tok::Num(_)) => {}
                    _ => self.tokens.push(Tok::Num(key.into())),
                }
            }
            "," => {
                self.start_fresh_if_evaluated(false);
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) => { if !n.contains('.') && !n.contains('e') { n.push('.'); } }
                    _ => self.tokens.push(Tok::Num("0.".into())),
                }
            }
            "+" | "−" | "×" | "÷" | "xʸ" => {
                let op = if key == "xʸ" { '^' } else { key.chars().next().unwrap() };
                self.start_fresh_if_evaluated(true);
                match self.tokens.last() {
                    Some(Tok::Op(prev)) => {
                        if op == '−' && matches!(prev, '×' | '÷' | '^') { self.tokens.push(Tok::Op('−')); }
                        else if self.tokens.len() >= 2 && matches!(self.tokens[self.tokens.len() - 2], Tok::Op(_)) { self.tokens.pop(); self.tokens.pop(); self.tokens.push(Tok::Op(op)); }
                        else if self.tokens.len() == 1 { if op == '−' { self.tokens[0] = Tok::Op('−'); } }
                        else { *self.tokens.last_mut().unwrap() = Tok::Op(op); }
                    }
                    None | Some(Tok::LParen) | Some(Tok::Func(_)) | Some(Tok::Equals) => { if op == '−' { self.tokens.push(Tok::Op('−')); } }
                    _ => self.tokens.push(Tok::Op(op)),
                }
            }
            "±" => {
                self.start_fresh_if_evaluated(true);
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) => { if n != "0" { *n = match n.strip_prefix('-') { Some(r) => r.to_string(), None => format!("-{n}") }; } }
                    _ => { if !is_value_end(self.tokens.last()) { self.tokens.push(Tok::Op('−')); } }
                }
            }
            "%" | "x²" | "1/x" | "n!" => {
                let p = match key { "%" => "%", "x²" => "²", "1/x" => "⁻¹", _ => "!" };
                self.start_fresh_if_evaluated(true);
                if is_value_end(self.tokens.last()) { self.tokens.push(Tok::Post(p)); }
            }
            "(" => self.push_value_tok(Tok::LParen),
            ")" => { if !self.just_evaluated && self.open_parens() > 0 && is_value_end(self.tokens.last()) { self.tokens.push(Tok::RParen); } }
            "π" => self.push_value_tok(Tok::Pi),
            "e" => self.push_value_tok(Tok::E),
            "x" => self.push_value_tok(Tok::X),
            "√" | "sin" | "cos" | "tan" | "ln" | "log" => {
                let f: &'static str = match key { "√" => "√", "sin" => "sin", "cos" => "cos", "tan" => "tan", "ln" => "ln", _ => "log" };
                self.push_value_tok(Tok::Func(f));
            }
            "=" => self.equals(),
            _ => {}
        }
    }

    fn equals(&mut self) {
        if self.just_evaluated || self.tokens.is_empty() { return; }
        let has_x = self.tokens.contains(&Tok::X);
        if has_x && !self.tokens.contains(&Tok::Equals) {
            // first "=" in an equation inserts the equals sign; the next one solves
            self.tokens.push(Tok::Equals);
            return;
        }
        let line = display_tokens(&self.tokens);
        if has_x {
            let (text, value) = match self.ev().solve(&self.tokens) {
                Ok(r) if r.len() == 1 => (format!("x = {}", display_num(r[0])), Some(r[0])),
                Ok(r) => (format!("x₁ = {};  x₂ = {}", display_num(r[0]), display_num(r[1])), None),
                Err(e) => (solve_error_text(&e).to_string(), None),
            };
            self.s.history.insert(0, HistItem { expr: line.clone(), result: text.clone(), value });
            self.result = Some((line, text));
            self.tokens = value.map(|v| vec![Tok::Num(plain(v))]).unwrap_or_default();
        } else {
            match self.ev().eval(&self.tokens) {
                Ok(v) => {
                    let text = display_num(v);
                    self.s.history.insert(0, HistItem { expr: line.clone(), result: text.clone(), value: Some(v) });
                    self.result = Some((format!("{line} ="), text));
                    self.tokens = vec![Tok::Num(plain(v))];
                }
                Err(e) => { self.result = Some((format!("{line} ="), expr::error_text(&e).to_string())); self.tokens.clear(); }
            }
        }
        self.s.history.truncate(200);
        self.just_evaluated = true;
    }

    pub fn memory(&mut self, key: &str) {
        let cur = self.current_value();
        match key {
            "MC" => self.s.memory = 0.0,
            "M+" => if let Some(v) = cur { self.s.memory += v; self.just_evaluated = true; },
            "M−" => if let Some(v) = cur { self.s.memory -= v; self.just_evaluated = true; },
            "MR" => {
                let m = self.s.memory;
                if self.just_evaluated || self.tokens.is_empty() { self.set_value(m); self.just_evaluated = false; }
                else if !is_value_end(self.tokens.last()) { self.tokens.push(Tok::Num(plain(m))); }
            }
            _ => {}
        }
    }

    pub fn paste(&mut self, text: &str) {
        if let Some(raw) = expr::parse_pasted(text) {
            if self.just_evaluated || self.tokens.is_empty() { self.tokens = vec![Tok::Num(raw)]; self.just_evaluated = false; self.result = None; }
            else if !is_value_end(self.tokens.last()) { self.tokens.push(Tok::Num(raw)); }
            else { self.tokens.push(Tok::Op('×')); self.tokens.push(Tok::Num(raw)); }
            self.toast = Some("Įklijuota".into());
        } else {
            self.toast = Some("Iškarpinėje nėra skaičiaus".into());
        }
    }

    pub fn handle(&mut self, a: &Action) -> Vec<Effect> {
        let mut fx = vec![];
        let save_before = (self.s.memory, self.s.history.len(), self.s.theme, self.s.vibrate, self.s.degrees, self.s.sci);
        self.popup = matches!(a, Action::DisplayLongPress);
        match a {
            Action::Key(k) => {
                if self.s.vibrate { fx.push(Effect::Vibrate); }
                if k.starts_with('M') { self.memory(k); }
                else if k == "DEG" || k == "RAD" { self.s.degrees = !self.s.degrees; }
                else { self.press(k); }
            }
            Action::Nav(s) => {
                self.screen = *s; self.scroll = 0.0;
                if *s == Screen::Converter && self.s.conv_cat == CURRENCY_CAT { fx.push(Effect::FetchRates); }
            }
            Action::Back => {
                self.scroll = 0.0;
                self.screen = match self.screen { Screen::PickUnit { .. } => Screen::Converter, Screen::Gallery => Screen::Photo, _ => Screen::Calc };
            }
            Action::UseValue(v) => { self.set_value(*v); self.screen = Screen::Calc; }
            Action::UseTokens => {
                if let PhotoState::Done { tokens, .. } = &self.photo { self.tokens = tokens.clone(); self.just_evaluated = false; self.result = None; }
                self.screen = Screen::Calc;
            }
            Action::LoadHistory(i) => {
                if let Some(h) = self.s.history.get(*i) { if let Some(v) = h.value { self.set_value(v); } }
                self.screen = Screen::Calc;
            }
            Action::ClearHistory => self.s.history.clear(),
            Action::ToggleSci => self.s.sci = !self.s.sci,
            Action::CycleTheme => self.s.theme = (self.s.theme + 1) % 3,
            Action::ToggleVibrate => self.s.vibrate = !self.s.vibrate,
            Action::ToggleAngle => self.s.degrees = !self.s.degrees,
            Action::CheckUpdate => { fx.push(Effect::CheckUpdate); self.toast = Some("Tikrinami naujiniai…".into()); }
            Action::UpdateBanner => fx.push(Effect::UpdateTap),
            Action::Copy => {
                let text = match self.current_value() { Some(v) => expr::clipboard_num(v), None => self.result.as_ref().map(|r| r.1.clone()).unwrap_or_default() };
                if !text.is_empty() { fx.push(Effect::Copy(text)); self.toast = Some("Nukopijuota".into()); }
            }
            Action::Paste => fx.push(Effect::RequestPaste),
            Action::DisplayLongPress | Action::ClosePopup | Action::None => {}
            Action::ConvCat(c) => { self.s.conv_cat = *c; if *c == CURRENCY_CAT { fx.push(Effect::FetchRates); } fx.push(Effect::Save); }
            Action::PickUnit(i) => {
                let c = self.s.conv_cat;
                if let Screen::PickUnit { from } = self.screen { if from { self.s.conv_from[c] = *i; } else { self.s.conv_to[c] = *i; } }
                self.screen = Screen::Converter; fx.push(Effect::Save);
            }
            Action::SwapUnits => { let c = self.s.conv_cat; let t = self.s.conv_from[c]; self.s.conv_from[c] = self.s.conv_to[c]; self.s.conv_to[c] = t; fx.push(Effect::Save); }
            Action::RefreshRates => fx.push(Effect::FetchRates),
            Action::Tip(i) => { self.s.tip = *i; fx.push(Effect::Save); }
            Action::People(n) => { self.s.people = *n; fx.push(Effect::Save); }
            Action::TakePhoto => fx.push(Effect::TakePhoto),
            Action::OpenGallery => { self.screen = Screen::Gallery; self.scroll = 0.0; fx.push(Effect::OpenGallery); }
            Action::GalleryPick(id) => { self.screen = Screen::Photo; self.photo = PhotoState::Working; fx.push(Effect::LoadGallery(*id)); }
        }
        let after = (self.s.memory, self.s.history.len(), self.s.theme, self.s.vibrate, self.s.degrees, self.s.sci);
        if after != save_before && !fx.contains(&Effect::Save) { fx.push(Effect::Save); }
        fx
    }

    /// Processes a decoded photo (runs OCR; call from a worker thread with a clone of the image).
    pub fn photo_result(img: image::RgbImage) -> PhotoState {
        match crate::ocr::recognize_lines(&img) {
            Ok(lines) => {
                let texts: Vec<String> = lines.iter().map(|l| l.text.clone()).collect();
                match crate::ocr::best_index(&texts) {
                    Some(i) => {
                        let tokens = crate::ocr::to_tokens(&texts[i]);
                        if tokens.is_empty() { return PhotoState::Failed("Nuotraukoje neradau uždavinio.".into()); }
                        PhotoState::Done { bbox: Some(lines[i].bbox), text: texts[i].clone(), tokens, img: thumb(&img, 900) }
                    }
                    None => PhotoState::Failed("Nuotraukoje neradau uždavinio. Fotografuokite arčiau, kad skaičiai būtų ryškūs.".into()),
                }
            }
            Err(e) => PhotoState::Failed(format!("Atpažinti nepavyko: {e}")),
        }
    }

    pub fn photo_answer(&self, tokens: &[Tok]) -> (String, Vec<String>) {
        let ev = self.ev();
        if tokens.contains(&Tok::X) {
            let mut steps = vec![];
            let pos = tokens.iter().position(|t| *t == Tok::Equals);
            let (l, r) = match pos { Some(p) => (&tokens[..p], &tokens[p + 1..]), None => (tokens, &[][..]) };
            if let (Ok(lp), Ok(rp)) = (ev.eval_poly(l), ev.eval_poly(r)) {
                let [c, b, a] = [lp.0[0] - rp.0[0], lp.0[1] - rp.0[1], lp.0[2] - rp.0[2]];
                if a.abs() < 1e-12 && b.abs() > 1e-12 {
                    steps.push(format!("Perkeliame narius: {}x = {}", display_num(b), display_num(-c)));
                    steps.push(format!("Daliname iš {}: x = {} ÷ {}", display_num(b), display_num(-c), display_num(b)));
                } else if a.abs() > 1e-12 {
                    steps.push(format!("Kvadratinė lygtis: a = {}, b = {}, c = {}", display_num(a), display_num(b), display_num(c)));
                    steps.push(format!("D = b² − 4ac = {}", display_num(b * b - 4.0 * a * c)));
                    steps.push("x = (−b ± √D) ÷ 2a".into());
                }
            }
            let ans = match ev.solve(tokens) {
                Ok(r) if r.len() == 1 => format!("x = {}", display_num(r[0])),
                Ok(r) => format!("x₁ = {};  x₂ = {}", display_num(r[0]), display_num(r[1])),
                Err(e) => solve_error_text(&e).to_string(),
            };
            (ans, steps)
        } else {
            match ev.eval(tokens) { Ok(v) => (format!("= {}", display_num(v)), vec![]), Err(e) => (expr::error_text(&e).to_string(), vec![]) }
        }
    }
}

pub fn thumb(img: &image::RgbImage, max: u32) -> image::RgbImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= max { return img.clone(); }
    image::imageops::thumbnail(img, (w as u64 * max as u64 / w.max(h) as u64) as u32, (h as u64 * max as u64 / w.max(h) as u64) as u32)
}

pub fn solve_error_text(e: &SolveError) -> &'static str {
    match e {
        SolveError::NoSolution => "Sprendinių nėra",
        SolveError::AllX => "Tinka bet koks x",
        SolveError::NoRealSolution => "Realių sprendinių nėra",
        SolveError::Calc(c) => expr::error_text(c),
    }
}

// ---------------- theme ----------------

pub struct Theme { pub bg: Rgb, pub key: Rgb, pub key_op: Rgb, pub key_eq: Rgb, pub key_eq_text: Rgb, pub key_text: Rgb, pub text: Rgb, pub dim: Rgb, pub chip: Rgb, pub chip_on: Rgb, pub chip_on_text: Rgb, pub pressed: Rgb, pub line: Rgb, pub shadow: Rgb }

pub fn theme(dark: bool) -> Theme {
    if dark {
        Theme { bg: [0x12, 0x12, 0x12], key: [0x2C, 0x2C, 0x2E], key_op: [0x3A, 0x3A, 0x3D], key_eq: [0xE8, 0xE8, 0xE8], key_eq_text: [0x11, 0x11, 0x11], key_text: [0xF0, 0xF0, 0xF0],
            text: [0xFF, 0xFF, 0xFF], dim: [0x9A, 0x9A, 0x9A], chip: [0x26, 0x26, 0x28], chip_on: [0xE8, 0xE8, 0xE8], chip_on_text: [0x11, 0x11, 0x11], pressed: [0x55, 0x55, 0x58], line: [0x33, 0x33, 0x33], shadow: [0x08, 0x08, 0x08] }
    } else {
        Theme { bg: [0xFF, 0xFF, 0xFF], key: [0xD6, 0xD7, 0xD7], key_op: [0xC4, 0xC6, 0xC6], key_eq: [0x22, 0x22, 0x22], key_eq_text: [0xFF, 0xFF, 0xFF], key_text: [0x21, 0x21, 0x21],
            text: [0x00, 0x00, 0x00], dim: [0x70, 0x70, 0x70], chip: [0xEE, 0xEE, 0xEE], chip_on: [0x22, 0x22, 0x22], chip_on_text: [0xFF, 0xFF, 0xFF], pressed: [0xA8, 0xA9, 0xA9], line: [0xE2, 0xE2, 0xE2], shadow: [0xB0, 0xB0, 0xB0] }
    }
}

// ---------------- widgets ----------------

#[derive(Clone, Copy, PartialEq)]
pub enum Kind { Digit, Op, Eq, Sci, Flat, Chip, ChipOn, Row, Invisible }

pub enum W {
    Fill { r: Rect, c: Rgb, radius: f32 },
    Text { r: Rect, text: String, px: f32, bold: bool, c: Rgb, align: u8, wrap: bool },
    Button { r: Rect, label: String, px: f32, kind: Kind, action: Action },
    Photo { r: Rect },
    Thumb { r: Rect, idx: usize },
}

pub struct Frame { pub widgets: Vec<W>, pub scroll_area: Option<Rect>, pub page: (usize, usize) }

impl App {
    pub fn build(&mut self, w: f32, h: f32, d: f32) -> Frame {
        let mut out = vec![];
        let t = theme(self.dark());
        out.push(W::Fill { r: Rect::new(0.0, 0.0, w, h), c: t.bg, radius: 0.0 });
        let mut scroll_area = None;
        let mut page = (0, 0);
        match self.screen {
            Screen::Calc => self.build_calc(&mut out, w, h, d, &t),
            _ => {
                let body = self.build_header(&mut out, w, d, &t);
                let area = Rect::new(0.0, body, w, h - body);
                scroll_area = Some(area);
                let start = out.len();
                let content_h = self.build_page(&mut out, area, d, &t);
                self.content_h = content_h;
                let max = (content_h - area.h).max(0.0);
                self.scroll = self.scroll.clamp(0.0, max);
                for wd in &mut out[start..] { shift(wd, -self.scroll); }
                page = (start, out.len());
            }
        }
        if let Some(b) = &self.banner {
            let r = Rect::new(12.0 * d, h - 60.0 * d, w - 24.0 * d, 48.0 * d);
            out.push(W::Fill { r, c: [0x22, 0x22, 0x22], radius: 10.0 * d });
            out.push(W::Text { r: r.inset(12.0 * d, 0.0), text: b.clone(), px: 14.0 * d, bold: false, c: [0xFF, 0xFF, 0xFF], align: 1, wrap: false });
            out.push(W::Button { r, label: String::new(), px: 0.0, kind: Kind::Invisible, action: Action::UpdateBanner });
        } else if let Some(msg) = &self.toast {
            let tw = 40.0 * d + msg.chars().count() as f32 * 8.0 * d;
            let r = Rect::new((w - tw) / 2.0, h - 90.0 * d, tw, 36.0 * d);
            out.push(W::Fill { r, c: [0x33, 0x33, 0x33], radius: 18.0 * d });
            out.push(W::Text { r, text: msg.clone(), px: 14.0 * d, bold: false, c: [0xFF, 0xFF, 0xFF], align: 1, wrap: false });
        }
        Frame { widgets: out, scroll_area, page }
    }

    fn build_calc(&self, out: &mut Vec<W>, w: f32, h: f32, d: f32, t: &Theme) {
        let landscape = w > h;
        let pad = 12.0 * d;
        // top bar
        let bar_h = 40.0 * d;
        let chips: [(&str, Action, bool); 6] = [
            ("Istorija", Action::Nav(Screen::History), false),
            ("Mokslinis", Action::ToggleSci, self.s.sci || landscape),
            ("Keitiklis", Action::Nav(Screen::Converter), false),
            ("PVM", Action::Nav(Screen::Vat), false),
            ("Foto", Action::Nav(Screen::Photo), false),
            ("⚙", Action::Nav(Screen::Settings), false),
        ];
        let gap = 6.0 * d;
        let cw = (w - 2.0 * pad - gap * 5.0) / 6.0;
        let fpx = (12.5 * d).min(cw / 6.2);
        for (i, (label, action, on)) in chips.iter().enumerate() {
            let r = Rect::new(pad + i as f32 * (cw + gap), pad, cw, bar_h - 8.0 * d);
            let px = if *label == "⚙" { 18.0 * d } else { fpx };
            out.push(W::Button { r, label: label.to_string(), px, kind: if *on { Kind::ChipOn } else { Kind::Chip }, action: action.clone() });
        }
        let top = pad + bar_h;
        let inner_w = w - 2.0 * pad;
        let avail = h - top - pad;
        let sci = self.s.sci || landscape;
        let mem_h = 38.0 * d;
        // display height
        let disp_h = if landscape { avail * 0.28 } else if sci { avail * 0.22 } else { avail * 0.26 };
        let disp = Rect::new(pad, top, inner_w, disp_h);
        self.build_display(out, disp, d, t);
        out.push(W::Button { r: disp, label: String::new(), px: 0.0, kind: Kind::Invisible, action: Action::ClosePopup });
        // memory row
        let mem = Rect::new(pad, disp.bottom(), inner_w, mem_h);
        let mkeys = ["MC", "M+", "M−", "MR"];
        let mw = mem.w / 4.0;
        for (i, k) in mkeys.iter().enumerate() {
            out.push(W::Button { r: Rect::new(mem.x + i as f32 * mw, mem.y, mw, mem.h), label: k.to_string(), px: 15.0 * d, kind: Kind::Flat, action: Action::Key(k.to_string()) });
        }
        let grid_top = mem.bottom() + 2.0 * d;
        let grid_h = h - pad - grid_top;
        let basic: [&str; 20] = ["C", "⌫", "÷", "×", "7", "8", "9", "−", "4", "5", "6", "+", "1", "2", "3", "=", "0", ",", "±", "%"];
        let deg = if self.s.degrees { "DEG" } else { "RAD" };
        let scik: [&str; 16] = ["(", ")", "x²", "xʸ", "√", "π", "e", "1/x", "sin", "cos", "tan", "n!", "ln", "log", "x", deg];
        let m = 3.0 * d;
        let place = |out: &mut Vec<W>, keys: &[&str], area: Rect, cols: usize, rows: usize, px: f32| {
            let (cw, rh) = (area.w / cols as f32, area.h / rows as f32);
            for (i, k) in keys.iter().enumerate() {
                let (c, r) = ((i % cols) as f32, (i / cols) as f32);
                let rect = Rect::new(area.x + c * cw + m, area.y + r * rh + m, cw - 2.0 * m, rh - 2.0 * m);
                let kind = match *k {
                    "=" => Kind::Eq,
                    "÷" | "×" | "−" | "+" | "C" | "⌫" | "%" | "±" => Kind::Op,
                    k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) || k == "," => Kind::Digit,
                    _ => Kind::Sci,
                };
                out.push(W::Button { r: rect, label: k.to_string(), px, kind, action: Action::Key(k.to_string()) });
            }
        };
        if landscape {
            let half = inner_w * 0.5;
            place(out, &scik, Rect::new(pad, grid_top, half, grid_h), 4, 4, 16.0 * d);
            place(out, &basic, Rect::new(pad + half, grid_top, half, grid_h), 4, 5, 18.0 * d);
        } else if sci {
            let sci_h = grid_h * 4.0 / 9.0 * 0.8;
            place(out, &scik, Rect::new(pad, grid_top, inner_w, sci_h), 4, 4, 17.0 * d);
            place(out, &basic, Rect::new(pad, grid_top + sci_h, inner_w, grid_h - sci_h), 4, 5, 22.0 * d);
        } else {
            place(out, &basic, Rect::new(pad, grid_top, inner_w, grid_h), 4, 5, 22.0 * d);
        }
        if self.popup {
            let pw = 240.0 * d;
            let r = Rect::new(disp.right() - pw, disp.y + 8.0 * d, pw, 44.0 * d);
            out.push(W::Fill { r: Rect { y: r.y + 2.0 * d, ..r }, c: t.shadow, radius: 10.0 * d });
            out.push(W::Fill { r, c: t.chip_on, radius: 10.0 * d });
            out.push(W::Button { r: Rect { w: pw / 2.0, ..r }, label: "Kopijuoti".into(), px: 15.0 * d, kind: Kind::ChipOn, action: Action::Copy });
            out.push(W::Button { r: Rect { x: r.x + pw / 2.0, w: pw / 2.0, ..r }, label: "Įklijuoti".into(), px: 15.0 * d, kind: Kind::ChipOn, action: Action::Paste });
        }
    }

    fn build_display(&self, out: &mut Vec<W>, r: Rect, d: f32, t: &Theme) {
        let inner = r.inset(8.0 * d, 4.0 * d);
        // indicators
        let mut ind = vec![];
        if self.s.memory != 0.0 { ind.push("M".to_string()); }
        if self.s.sci { ind.push(if self.s.degrees { "DEG" } else { "RAD" }.to_string()); }
        if !ind.is_empty() {
            out.push(W::Text { r: Rect::new(inner.x, inner.y, inner.w, 18.0 * d), text: ind.join("  "), px: 12.0 * d, bold: true, c: t.dim, align: 0, wrap: false });
        }
        let (small, big, big_bold) = match &self.result {
            Some((line, res)) => (line.clone(), res.clone(), true),
            None => {
                let e = display_tokens(&self.tokens);
                let e = if e.is_empty() { "0".to_string() } else { e };
                let has_op = self.tokens.iter().any(|t| !matches!(t, Tok::Num(_)));
                let prev = if has_op && !self.tokens.contains(&Tok::X) { self.current_value().map(|v| format!("= {}", display_num(v))).unwrap_or_default() } else { String::new() };
                (prev, e, !has_op)
            }
        };
        let (big_r, small_r) = if self.result.is_some() {
            (Rect::new(inner.x, inner.y + inner.h * 0.35, inner.w, inner.h * 0.65), Rect::new(inner.x, inner.y + 16.0 * d, inner.w, inner.h * 0.3))
        } else {
            (Rect::new(inner.x, inner.y + 16.0 * d, inner.w, inner.h * 0.7 - 16.0 * d), Rect::new(inner.x, inner.y + inner.h * 0.7, inner.w, inner.h * 0.3))
        };
        out.push(W::Text { r: small_r, text: small, px: 22.0 * d, bold: false, c: t.dim, align: 2, wrap: false });
        out.push(W::Text { r: big_r, text: big, px: 44.0 * d, bold: big_bold, c: t.text, align: 2, wrap: true });
    }

    /// Returns the y where the page body starts.
    fn build_header(&self, out: &mut Vec<W>, w: f32, d: f32, t: &Theme) -> f32 {
        let title = match self.screen {
            Screen::History => "Istorija", Screen::Vat => "PVM ir arbatpinigiai", Screen::Converter => "Keitiklis",
            Screen::PickUnit { .. } => "Pasirinkite vienetą", Screen::Settings => "Nustatymai", Screen::Photo => "Foto uždavinys",
            Screen::Gallery => "Pasirinkite nuotrauką", Screen::Calc => "",
        };
        let h = 56.0 * d;
        out.push(W::Button { r: Rect::new(4.0 * d, 4.0 * d, 48.0 * d, 48.0 * d), label: "‹".into(), px: 30.0 * d, kind: Kind::Flat, action: Action::Back });
        out.push(W::Text { r: Rect::new(56.0 * d, 0.0, w - 170.0 * d, h), text: title.into(), px: 19.0 * d, bold: true, c: t.text, align: 0, wrap: false });
        if self.screen == Screen::History && !self.s.history.is_empty() {
            out.push(W::Button { r: Rect::new(w - 110.0 * d, 10.0 * d, 98.0 * d, 36.0 * d), label: "Išvalyti".into(), px: 14.0 * d, kind: Kind::Chip, action: Action::ClearHistory });
        }
        out.push(W::Fill { r: Rect::new(0.0, h, w, 1.0 * d), c: t.line, radius: 0.0 });
        h + 1.0 * d
    }

    /// Lays out the page body starting at area.y; returns content height.
    fn build_page(&self, out: &mut Vec<W>, area: Rect, d: f32, t: &Theme) -> f32 {
        let pad = 16.0 * d;
        let x = area.x + pad;
        let w = area.w - 2.0 * pad;
        let mut y = area.y + 8.0 * d;
        let row = |out: &mut Vec<W>, y: &mut f32, label: &str, value: &str, action: Action| {
            let r = Rect::new(x, *y, w, 56.0 * d);
            out.push(W::Button { r, label: String::new(), px: 0.0, kind: Kind::Row, action });
            out.push(W::Text { r: Rect::new(r.x + 12.0 * d, r.y, r.w * 0.55, r.h), text: label.into(), px: 15.0 * d, bold: false, c: t.dim, align: 0, wrap: false });
            out.push(W::Text { r: Rect::new(r.x + r.w * 0.4, r.y, r.w * 0.6 - 12.0 * d, r.h), text: value.into(), px: 17.0 * d, bold: true, c: t.text, align: 2, wrap: false });
            *y += 60.0 * d;
        };
        let heading = |out: &mut Vec<W>, y: &mut f32, text: &str| {
            *y += 10.0 * d;
            out.push(W::Text { r: Rect::new(x, *y, w, 28.0 * d), text: text.into(), px: 14.0 * d, bold: true, c: t.dim, align: 0, wrap: false });
            *y += 32.0 * d;
        };
        let para = |out: &mut Vec<W>, y: &mut f32, text: &str, px: f32| {
            let lines = (text.chars().count() as f32 * px * 0.55 / w).ceil().max(1.0) + text.matches('\n').count() as f32;
            let hh = lines * px * 1.35 + 6.0 * d;
            out.push(W::Text { r: Rect::new(x, *y, w, hh), text: text.into(), px, bold: false, c: t.dim, align: 0, wrap: true });
            *y += hh + 4.0 * d;
        };
        let chips = |out: &mut Vec<W>, y: &mut f32, items: &[(String, Action, bool)]| {
            let gap = 8.0 * d;
            let mut cx = x;
            for (label, action, on) in items {
                let cw = 28.0 * d + label.chars().count() as f32 * 8.5 * d;
                if cx + cw > x + w { cx = x; *y += 44.0 * d; }
                out.push(W::Button { r: Rect::new(cx, *y, cw, 36.0 * d), label: label.clone(), px: 14.0 * d, kind: if *on { Kind::ChipOn } else { Kind::Chip }, action: action.clone() });
                cx += cw + gap;
            }
            *y += 48.0 * d;
        };
        let value = self.current_value();
        match self.screen {
            Screen::History => {
                if self.s.history.is_empty() { para(out, &mut y, "Istorija tuščia. Čia matysite atliktus skaičiavimus.", 15.0 * d); }
                for (i, hi) in self.s.history.iter().enumerate() {
                    let r = Rect::new(x, y, w, 64.0 * d);
                    out.push(W::Button { r, label: String::new(), px: 0.0, kind: Kind::Row, action: Action::LoadHistory(i) });
                    out.push(W::Text { r: Rect::new(r.x + 12.0 * d, r.y + 4.0 * d, r.w - 24.0 * d, 26.0 * d), text: hi.expr.clone(), px: 14.0 * d, bold: false, c: t.dim, align: 2, wrap: false });
                    out.push(W::Text { r: Rect::new(r.x + 12.0 * d, r.y + 28.0 * d, r.w - 24.0 * d, 32.0 * d), text: hi.result.clone(), px: 20.0 * d, bold: true, c: t.text, align: 2, wrap: false });
                    y += 68.0 * d;
                }
            }
            Screen::Vat => {
                let v = value.unwrap_or(0.0);
                row(out, &mut y, "Suma (iš skaičiuotuvo)", &display_num(v), Action::None);
                heading(out, &mut y, "PVM 21 %");
                row(out, &mut y, "Su PVM (+21 %)", &display_num(v * 1.21), Action::UseValue(v * 1.21));
                row(out, &mut y, "Be PVM (suma su PVM)", &display_num(v / 1.21), Action::UseValue(v / 1.21));
                row(out, &mut y, "PVM nuo sumos", &display_num(v * 0.21), Action::UseValue(v * 0.21));
                row(out, &mut y, "PVM sumoje su PVM", &display_num(v - v / 1.21), Action::UseValue(v - v / 1.21));
                heading(out, &mut y, "Arbatpinigiai");
                let tips = [5.0, 10.0, 15.0, 20.0];
                chips(out, &mut y, &tips.iter().enumerate().map(|(i, p)| (format!("{} %", *p as i32), Action::Tip(i), self.s.tip == i)).collect::<Vec<_>>());
                let p = tips[self.s.tip.min(3)] / 100.0;
                row(out, &mut y, "Arbatpinigiai", &display_num(v * p), Action::UseValue(v * p));
                row(out, &mut y, "Iš viso", &display_num(v * (1.0 + p)), Action::UseValue(v * (1.0 + p)));
                heading(out, &mut y, "Dalinti asmenims");
                chips(out, &mut y, &(1..=6).map(|n| (n.to_string(), Action::People(n), self.s.people == n)).collect::<Vec<_>>());
                let each = v * (1.0 + p) / self.s.people.max(1) as f64;
                row(out, &mut y, "Vienam asmeniui", &display_num(each), Action::UseValue(each));
                para(out, &mut y, "Bakstelėkite eilutę, kad rezultatą perkeltumėte į skaičiuotuvą.", 13.0 * d);
            }
            Screen::Converter => {
                let c = self.s.conv_cat;
                chips(out, &mut y, &CATEGORIES.iter().enumerate().map(|(i, cat)| (cat.name.to_string(), Action::ConvCat(i), i == c)).collect::<Vec<_>>());
                let names = convert::unit_names(c, &self.s.rates);
                let (fi, ti) = (self.s.conv_from[c].min(names.len().saturating_sub(1)), self.s.conv_to[c].min(names.len().saturating_sub(1)));
                let v = value.unwrap_or(1.0);
                row(out, &mut y, &format!("Iš: {}  ▾", names.get(fi).cloned().unwrap_or_default()), &display_num(v), Action::Nav(Screen::PickUnit { from: true }));
                out.push(W::Button { r: Rect::new(x + w / 2.0 - 30.0 * d, y - 6.0 * d, 60.0 * d, 36.0 * d), label: "⇅".into(), px: 20.0 * d, kind: Kind::Chip, action: Action::SwapUnits });
                y += 34.0 * d;
                let res = convert::convert(c, fi, ti, v, &self.s.rates);
                row(out, &mut y, &format!("Į: {}  ▾", names.get(ti).cloned().unwrap_or_default()), &res.map(display_num).unwrap_or("—".into()), Action::Nav(Screen::PickUnit { from: false }));
                if let Some(r) = res { out.push(W::Button { r: Rect::new(x, y, w, 40.0 * d), label: "Perkelti rezultatą į skaičiuotuvą".into(), px: 14.0 * d, kind: Kind::Chip, action: Action::UseValue(r) }); y += 52.0 * d; }
                if c == CURRENCY_CAT {
                    let note = match &self.s.rates {
                        Some(r) => format!("Europos centrinio banko kursai, {}. {}", r.date, self.rates_status),
                        None => format!("Kursai dar neatsisiųsti. Reikia interneto. {}", self.rates_status),
                    };
                    para(out, &mut y, &note, 13.0 * d);
                    out.push(W::Button { r: Rect::new(x, y, 160.0 * d, 36.0 * d), label: "Atnaujinti kursus".into(), px: 14.0 * d, kind: Kind::Chip, action: Action::RefreshRates });
                    y += 48.0 * d;
                }
                para(out, &mut y, "Reikšmė imama iš skaičiuotuvo ekrano.", 13.0 * d);
            }
            Screen::PickUnit { from } => {
                let c = self.s.conv_cat;
                let sel = if from { self.s.conv_from[c] } else { self.s.conv_to[c] };
                for (i, n) in convert::unit_names(c, &self.s.rates).iter().enumerate() {
                    row(out, &mut y, n, if i == sel { "✓" } else { "" }, Action::PickUnit(i));
                }
            }
            Screen::Settings => {
                row(out, &mut y, "Tema", ["Automatinė", "Šviesi", "Tamsi"][self.s.theme as usize % 3], Action::CycleTheme);
                row(out, &mut y, "Vibracija paspaudus", if self.s.vibrate { "Įjungta" } else { "Išjungta" }, Action::ToggleVibrate);
                row(out, &mut y, "Kampų vienetai", if self.s.degrees { "Laipsniai" } else { "Radianai" }, Action::ToggleAngle);
                row(out, &mut y, "Mokslinis režimas", if self.s.sci { "Įjungtas" } else { "Išjungtas" }, Action::ToggleSci);
                row(out, &mut y, "Tikrinti naujinius", "Tikrinti", Action::CheckUpdate);
                row(out, &mut y, "Versija", version(), Action::None);
                para(out, &mut y, &format!("Naujiniai: github.com/{}\nSkaičiuotuvas parašytas Rust kalba.", crate::update::UPDATE_REPO), 13.0 * d);
            }
            Screen::Photo => {
                let bw = (w - 10.0 * d) / 2.0;
                out.push(W::Button { r: Rect::new(x, y, bw, 48.0 * d), label: "Fotografuoti".into(), px: 16.0 * d, kind: Kind::ChipOn, action: Action::TakePhoto });
                out.push(W::Button { r: Rect::new(x + bw + 10.0 * d, y, bw, 48.0 * d), label: "Iš galerijos".into(), px: 16.0 * d, kind: Kind::Chip, action: Action::OpenGallery });
                y += 60.0 * d;
                match &self.photo {
                    PhotoState::Idle => para(out, &mut y, "Nufotografuokite spausdintą uždavinį, pvz. „2x + 3 = 11“ ar „125 : 5 =“. Programa suras uždavinį nuotraukoje ir jį išspręs.\n\nVeikia be interneto. Ranka rašyto teksto kol kas neatpažįsta.", 15.0 * d),
                    PhotoState::Working => para(out, &mut y, "Atpažįstama… (kelios sekundės)", 16.0 * d),
                    PhotoState::Failed(m) => para(out, &mut y, m, 15.0 * d),
                    PhotoState::Done { text, tokens, img, .. } => {
                        let ih = (w * img.height() as f32 / img.width().max(1) as f32).min(260.0 * d);
                        out.push(W::Photo { r: Rect::new(x, y, w, ih) });
                        y += ih + 10.0 * d;
                        row(out, &mut y, "Atpažinta", &text.clone(), Action::None);
                        row(out, &mut y, "Uždavinys", &display_tokens(tokens), Action::UseTokens);
                        let (ans, steps) = self.photo_answer(tokens);
                        for s in &steps { para(out, &mut y, s, 14.0 * d); }
                        out.push(W::Text { r: Rect::new(x, y, w, 50.0 * d), text: ans, px: 26.0 * d, bold: true, c: t.text, align: 0, wrap: true });
                        y += 58.0 * d;
                        out.push(W::Button { r: Rect::new(x, y, w, 44.0 * d), label: "Taisyti skaičiuotuve".into(), px: 15.0 * d, kind: Kind::Chip, action: Action::UseTokens });
                        y += 52.0 * d;
                        para(out, &mut y, "Jei atpažinta neteisingai, bakstelėkite „Taisyti skaičiuotuve“, pataisykite ir spauskite „=“.", 13.0 * d);
                    }
                }
            }
            Screen::Gallery => {
                if !self.gallery_status.is_empty() { para(out, &mut y, &self.gallery_status, 14.0 * d); }
                let cols = 3usize;
                let gap = 6.0 * d;
                let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
                for (i, it) in self.gallery.iter().enumerate() {
                    let (c, r) = ((i % cols) as f32, (i / cols) as f32);
                    let rect = Rect::new(x + c * (cw + gap), y + r * (cw + gap), cw, cw);
                    out.push(W::Fill { r: rect, c: t.chip, radius: 6.0 * d });
                    if it.thumb.is_some() { out.push(W::Thumb { r: rect, idx: i }); }
                    else { out.push(W::Text { r: rect.inset(6.0 * d, 0.0), text: it.label.clone(), px: 12.0 * d, bold: false, c: t.dim, align: 1, wrap: true }); }
                    out.push(W::Button { r: rect, label: String::new(), px: 0.0, kind: Kind::Invisible, action: Action::GalleryPick(it.id) });
                }
                y += ((self.gallery.len() + cols - 1) / cols) as f32 * (cw + gap);
            }
            Screen::Calc => {}
        }
        y - area.y + 24.0 * d
    }
}

fn shift(w: &mut W, dy: f32) {
    match w {
        W::Fill { r, .. } | W::Text { r, .. } | W::Button { r, .. } | W::Photo { r } | W::Thumb { r, .. } => r.y += dy,
    }
}

impl Frame {
    pub fn hit(&self, x: f32, y: f32) -> Option<usize> {
        // topmost button wins
        for (i, w) in self.widgets.iter().enumerate().rev() {
            if let W::Button { r, .. } = w {
                if r.contains(x, y) {
                    if let Some(a) = self.scroll_area { if i >= self.page.0 && i < self.page.1 && !a.contains(x, y) { continue; } }
                    return Some(i);
                }
            }
        }
        None
    }
    pub fn action(&self, i: usize) -> Option<Action> { match self.widgets.get(i) { Some(W::Button { action, .. }) => Some(action.clone()), _ => None } }
}

pub fn draw(app: &App, frame: &Frame, fonts: &Fonts, cv: &mut Canvas, d: f32, pressed: Option<usize>) {
    let t = theme(app.dark());
    let full = Rect::new(0.0, 0.0, cv.w as f32, cv.h as f32);
    for (i, wd) in frame.widgets.iter().enumerate() {
        let clip = match frame.scroll_area { Some(a) if i >= frame.page.0 && i < frame.page.1 => a, _ => full };
        let is_pressed = pressed == Some(i);
        match wd {
            W::Fill { r, c, radius } => cv.round_rect(*r, *radius, *c, clip),
            W::Text { r, text, px, bold, c, align, wrap } => text_in(cv, fonts, *r, text, *px, *bold, *c, *align, *wrap, clip),
            W::Button { r, label, px, kind, .. } => {
                let (bg, fg, radius, shadow) = match kind {
                    Kind::Digit => (t.key, t.key_text, 4.0 * d, true),
                    Kind::Op => (t.key_op, t.key_text, 4.0 * d, true),
                    Kind::Sci => (t.key_op, t.key_text, 4.0 * d, false),
                    Kind::Eq => (t.key_eq, t.key_eq_text, 4.0 * d, true),
                    Kind::Flat => (t.bg, t.key_text, 8.0 * d, false),
                    Kind::Chip => (t.chip, t.key_text, 10.0 * d, false),
                    Kind::ChipOn => (t.chip_on, t.chip_on_text, 10.0 * d, false),
                    Kind::Row => (t.chip, t.key_text, 10.0 * d, false),
                    Kind::Invisible => (t.bg, t.key_text, 0.0, false),
                };
                if *kind == Kind::Invisible { if is_pressed { cv.round_rect(*r, 8.0 * d, t.line, clip); } continue; }
                let face = if matches!(kind, Kind::Digit | Kind::Op | Kind::Eq | Kind::Sci) { r.inset(2.0 * d, 3.0 * d) } else { *r };
                if shadow { cv.round_rect(Rect { y: face.y + 1.0 * d, ..face }, radius, t.shadow, clip); }
                let bg = if is_pressed { t.pressed } else { bg };
                cv.round_rect(face, radius, bg, clip);
                if !label.is_empty() {
                    let mut px = *px;
                    let tw = fonts.width(label, px, false);
                    if tw > face.w - 8.0 * d { px *= (face.w - 8.0 * d) / tw; }
                    text_in(cv, fonts, face, label, px, false, fg, 1, false, clip);
                }
            }
            W::Photo { r } => {
                if let PhotoState::Done { img, bbox, .. } = &app.photo {
                    let dst = cv.image(img, *r, clip);
                    if let Some(b) = bbox {
                        // highlight where the problem was found
                        let s = dst.w / img.width().max(1) as f32;
                        let (sx, sy) = (dst.x, dst.y);
                        // bbox is in the downscaled OCR image; the stored image uses the same scale basis via thumb()
                        let k = img.width() as f32 / app_photo_src_w(app).max(1.0);
                        let rr = Rect::new(sx + b[0] * k * s - 4.0 * d, sy + b[1] * k * s - 4.0 * d, (b[2] - b[0]) * k * s + 8.0 * d, (b[3] - b[1]) * k * s + 8.0 * d);
                        cv.outline(rr, 3.0 * d, [0xFF, 0x3B, 0x30], clip);
                    }
                }
            }
            W::Thumb { r, idx } => { if let Some(Some(th)) = app.gallery.get(*idx).map(|g| g.thumb.as_ref()) { cv.image(th, r.inset(2.0 * d, 2.0 * d), clip); } }
        }
    }
}

/// Width of the image the OCR ran on (the bbox coordinates refer to it).
pub static PHOTO_SRC_W: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
fn app_photo_src_w(_app: &App) -> f32 { PHOTO_SRC_W.load(std::sync::atomic::Ordering::Relaxed) as f32 }

pub fn text_in(cv: &mut Canvas, fonts: &Fonts, r: Rect, text: &str, px: f32, bold: bool, c: Rgb, align: u8, wrap: bool, clip: Rect) {
    if text.is_empty() { return; }
    let clip = clip.intersect(&r.inset(-2.0, -2.0));
    let mut px = px;
    let mut lines = vec![text.to_string()];
    if wrap {
        // shrink down to 60 % before wrapping (keeps long numbers on one line when possible)
        let w0 = fonts.width(text, px, bold);
        if w0 > r.w { px = (px * r.w / w0).max(px * 0.6); }
        lines = fonts.wrap(text, px, bold, r.w);
    } else {
        let w0 = fonts.width(text, px, bold);
        if w0 > r.w && align != 0 { px *= r.w / w0; }
    }
    let (asc, desc, lh) = fonts.line(px, bold);
    let total = lh * (lines.len() as f32 - 1.0) + (asc - desc);
    let mut base = r.y + (r.h - total) / 2.0 + asc;
    if wrap && total > r.h { base = r.bottom() - total + asc; } // keep the end (latest digits) visible
    for line in &lines {
        let w = fonts.width(line, px, bold);
        let x = match align { 0 => r.x, 1 => r.x + (r.w - w) / 2.0, _ => r.right() - w };
        fonts.draw(cv, line, px, bold, x, base, c, clip);
        base += lh;
    }
}
