//! Application state, key handling and screen layout (as a list of widgets).

use crate::convert::{self, Rates, CATEGORIES, CURRENCY_CAT};
use crate::draw::{Canvas, Fonts, Rect, Rgb};
use crate::expr::{self, display_num, display_tokens, plain, Evaluator, SolveError, Tok};
use serde::{Deserialize, Serialize};

pub const VERSION: &str = include_str!("../VERSION");
pub fn version() -> &'static str { VERSION.trim() }

// ---------------- persistent data ----------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistItem { pub expr: String, pub result: String, pub value: Option<f64>, #[serde(default)] pub pinned: bool, #[serde(default)] pub note: String, #[serde(default)] pub photo: bool }
impl HistItem { pub fn new(expr: String, result: String, value: Option<f64>) -> Self { HistItem { expr, result, value, pinned: false, note: String::new(), photo: false } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub theme: u8,       // 0 auto, 1 light, 2 dark
    pub vibrate: bool,
    pub degrees: bool,
    pub sci: bool,
    pub memory: f64,     // (old single memory; migrated into mem[0])
    pub history: Vec<HistItem>,
    pub conv_cat: usize,
    pub conv_from: Vec<usize>,
    pub conv_to: Vec<usize>,
    pub rates: Option<Rates>,
    pub tip: usize,
    pub people: usize,
    pub ans: f64,
    pub accent: u8,      // 0 classic, 1 blue, 2 green, 3 orange, 4 purple, 5 red, 6 phone colors (Material You)
    pub key_size: u8,    // 0 normal, 1 large, 2 small
    pub font_size: u8,   // 0 normal, 1 large, 2 extra large, 3 small
    pub animations: bool,
    pub sound: bool,
    pub one_hand: u8,    // 0 off, 1 right hand, 2 left hand
    pub icon: u8,        // 0 classic, 1 blue, 2 dark
    pub contrast: bool,
    pub start_screen: u8, // 0 calculator, 1 converter, 2 tools, 3 history
    pub speak: bool,
    pub beta: bool,
    pub bg_check: bool,
    pub vib_ms: u8,      // vibration length in ms
    pub last_version: String,
    pub fav_cats: Vec<usize>,
    pub mem: Vec<f64>,   // M1-M5
    pub mem_slot: usize,
    pub decimal: u8,     // 0 comma, 1 dot
    pub grouping: u8,    // 0 space, 1 none, 2 dot/comma, 3 apostrophe
    pub fixed: i32,      // -1 automatic, else digits after the decimal sign
    pub words: bool,
    pub lang: i32,       // -1 = phone language, otherwise index into i18n::LANGS (0 = English)
}

impl Default for Saved {
    fn default() -> Self {
        Saved { theme: 0, vibrate: true, degrees: true, sci: false, memory: 0.0, history: vec![], conv_cat: 1,
            conv_from: vec![0, 3, 2, 0, 1, 1, 1], conv_to: vec![1, 7, 5, 1, 3, 3, 2], rates: None, tip: 1, people: 1, ans: 0.0, accent: 0, key_size: 0, font_size: 0, animations: true, sound: false, one_hand: 0, icon: 0, contrast: false, start_screen: 0, speak: false, beta: false, bg_check: true, vib_ms: 20, last_version: String::new(), fav_cats: vec![], mem: vec![0.0; 5], mem_slot: 0, decimal: 0, grouping: 0, fixed: -1, words: false, lang: 0 }
    }
}

// ---------------- runtime state ----------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen { Calc, History, Vat, Converter, PickUnit { from: bool }, Settings, Photo, Gallery, Tools, Tool, Graph, Ask }

pub enum PhotoState {
    Idle,
    Working,
    Done { img: image::RgbImage, bbox: Option<[f32; 4]>, text: String, tokens: Vec<Tok>, others: Vec<(String, [f32; 4])> },
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
    CycleLang,
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
    Set(&'static str),
    OpenTool(usize),
    ToolMode(usize),
    Focus(usize),
    FormKey(&'static str),
    Swipe(bool, Box<Action>),
    TextKey(char),
    TextBack,
    StartText(u8, usize),
    PhotoPick(usize),
    EndText,
    Pin(usize),
    DeleteHistory(usize),
    HwKey(char),
    None,
}

/// Side effects the platform layer performs.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect { Floating, BgCheck(bool), Listen, Solve(String, bool), NanoStatus, ReportBug, SetIcon(u8), Click, Keyboard(bool), Share(String), SaveFile(String, String), Speak(String), Vibrate, Copy(String), RequestPaste, Save, FetchRates, FetchRateHistory, CheckUpdate, UpdateTap, TakePhoto, OpenGallery, LoadGallery(i64) }

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
    pub system_lang: usize,
    pub photo: PhotoState,
    pub gallery: Vec<GalleryItem>,
    pub gallery_status: String,
    pub rates_status: String,
    pub cursor: Option<usize>,
    pub undo: Vec<Vec<Tok>>,
    pub redo: Vec<Vec<Tok>>,
    pub show_frac: bool,
    pub sci_page: u8,
    pub tool: usize,
    pub tool_mode: usize,
    pub fields: Vec<String>,
    pub focus: usize,
    pub list: Vec<f64>,
    pub seed: u64,
    pub graph_span: f64,
    pub text_target: Option<(u8, usize)>, // (0 = history search, 1 = note for entry)
    pub search: String,
    pub ask_text: String,
    pub ask_out: Option<(String, String, bool)>, // expression, answer, solved by Gemini Nano
    pub ask_status: String,
    pub nano: String, // "available", "downloadable", "downloading", "unavailable", "" = unknown
    pub restore_pending: bool,
    pub system_accent: Option<Rgb>,
    pub system_font: f32,
    pub whats_new: bool,
    pub crash: Option<String>, // error text from the last crash, shown once on start
    pub rate_hist: Option<(String, String, Vec<(String, f64)>)>,
}

impl Default for App { fn default() -> Self { Self::new(Saved::default()) } }

fn is_value_end(t: Option<&Tok>) -> bool { matches!(t, Some(Tok::Num(_) | Tok::RParen | Tok::Pi | Tok::E | Tok::X | Tok::I | Tok::Ans | Tok::Post(_))) }

impl App {
    pub fn new(s: Saved) -> Self {
        let mut s = s;
        if s.mem.len() < 5 { s.mem.resize(5, 0.0); }
        if s.memory != 0.0 && s.mem[0] == 0.0 { s.mem[0] = s.memory; s.memory = 0.0; }
        s.mem_slot = s.mem_slot.min(4);
        for (i, (f, t)) in convert::DEFAULTS.iter().enumerate().take(CATEGORIES.len()) { if s.conv_from.len() <= i { s.conv_from.push(*f); } if s.conv_to.len() <= i { s.conv_to.push(*t); } }
        s.conv_cat = s.conv_cat.min(CATEGORIES.len() - 1);
        let a = App { s, tokens: vec![], just_evaluated: false, result: None, screen: Screen::Calc, scroll: 0.0, content_h: 0.0,
            popup: false, banner: None, toast: None, system_dark: false, system_lang: 0, photo: PhotoState::Idle, gallery: vec![],
            gallery_status: String::new(), rates_status: String::new(), cursor: None, undo: vec![], redo: vec![], show_frac: false, sci_page: 0, tool: 0, tool_mode: 0, fields: vec![], focus: 0, list: vec![], seed: 0x9E3779B97F4A7C15, graph_span: 10.0, rate_hist: None, text_target: None, search: String::new(), ask_text: String::new(), ask_out: None, ask_status: String::new(), nano: String::new(), restore_pending: false, system_accent: None, system_font: 1.0, whats_new: false, crash: None };
        a.apply_lang();
        a.apply_format();
        let mut a = a;
        a.screen = match a.s.start_screen { 1 => Screen::Converter, 2 => Screen::Tools, 3 => Screen::History, _ => Screen::Calc };
        // "What's new" after an update (not on the very first start)
        if a.s.last_version != version() { a.whats_new = !a.s.last_version.is_empty(); a.s.last_version = version().to_string(); }
        a.apply_look();
        a
    }

    pub fn apply_look(&self) {
        crate::update::BETA.store(self.s.beta, std::sync::atomic::Ordering::Relaxed);
        let acc: Option<Rgb> = match self.s.accent { 1 => Some([0x1E, 0x6F, 0xD9]), 2 => Some([0x2E, 0x8B, 0x57]), 3 => Some([0xF2, 0x8C, 0x28]), 4 => Some([0x7E, 0x57, 0xC2]), 5 => Some([0xD3, 0x2F, 0x2F]), 6 => self.system_accent.or(Some([0x1E, 0x6F, 0xD9])), _ => None };
        LOOK.store(acc.map(|c| 0x0100_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32).unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
        CONTRAST.store(self.s.contrast, std::sync::atomic::Ordering::Relaxed);
        let user = [1.0, 1.15, 1.3, 0.9][self.s.font_size as usize % 4];
        FONT_SCALE.store(((user * self.system_font.clamp(0.85, 1.3)) * 1000.0) as u32, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn apply_format(&self) { expr::set_format(self.s.decimal, self.s.grouping, self.s.fixed); }
    pub fn apply_lang(&self) { crate::i18n::set_lang(if self.s.lang < 0 { self.system_lang } else { self.s.lang as usize }); }
    pub fn dark(&self) -> bool { match self.s.theme { 1 => false, 2 => true, _ => self.system_dark } }
    fn ev(&self) -> Evaluator { Evaluator { degrees: self.s.degrees, ans: self.s.ans, xval: None } }

    /// The value the calculator currently shows (result, or live value of the expression).
    pub fn current_value(&self) -> Option<f64> {
        if self.tokens.iter().any(|t| matches!(t, Tok::X | Tok::Equals | Tok::Cmp(_))) { return None; }
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
            "◀" => { let n = self.tokens.len(); let c = self.cursor.unwrap_or(n); self.cursor = Some(c.saturating_sub(1)); self.just_evaluated = false; self.result = None; return; }
            "▶" => { let n = self.tokens.len(); self.cursor = match self.cursor { Some(c) if c + 1 < n => Some(c + 1), _ => None }; return; }
            "↶" => { if let Some(p) = self.undo.pop() { self.redo.push(std::mem::replace(&mut self.tokens, p)); self.cursor = None; self.result = None; self.just_evaluated = false; } return; }
            "↷" => { if let Some(p) = self.redo.pop() { self.undo.push(std::mem::replace(&mut self.tokens, p)); self.cursor = None; self.result = None; self.just_evaluated = false; } return; }
            "a/b" => { self.show_frac = !self.show_frac; return; }
            "2nd" => { self.sci_page = (self.sci_page + 1) % 2; return; }
            _ => {}
        }
        let before = self.tokens.clone();
        match self.cursor {
            Some(c) if c < self.tokens.len() && key != "=" && key != "C" => {
                let after = self.tokens.split_off(c);
                let ev = self.just_evaluated; self.just_evaluated = false;
                self.press_inner(key);
                self.just_evaluated = ev && false;
                let nc = self.tokens.len();
                self.tokens.extend(after);
                self.cursor = Some(nc);
            }
            _ => { self.cursor = None; self.press_inner(key); }
        }
        if self.tokens != before { self.undo.push(before); if self.undo.len() > 100 { self.undo.remove(0); } self.redo.clear(); }
        if key != "=" { self.show_frac = false; }
    }

    fn press_inner(&mut self, key: &str) {
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
                if let Some(Tok::Num(n)) = self.tokens.last_mut() { if n.ends_with('e') { n.push('-'); return; } }
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
            "Ans" => self.push_value_tok(Tok::Ans),
            "i" => self.push_value_tok(Tok::I),
            "EE" => {
                self.start_fresh_if_evaluated(true);
                match self.tokens.last_mut() { Some(Tok::Num(n)) if !n.contains('e') => n.push('e'), _ => {} }
            }
            ";" => { if self.open_parens() > 0 && is_value_end(self.tokens.last()) { self.tokens.push(Tok::Sep); } }
            "mod" => { self.start_fresh_if_evaluated(true); if is_value_end(self.tokens.last()) { self.tokens.push(Tok::Op('m')); } }
            "<" | ">" | "≤" | "≥" => {
                if is_value_end(self.tokens.last()) && !self.tokens.iter().any(|t| matches!(t, Tok::Equals | Tok::Cmp(_))) { self.just_evaluated = false; self.result = None; self.tokens.push(Tok::Cmp(key.chars().next().unwrap())); }
            }
            "sin⁻¹" | "cos⁻¹" | "tan⁻¹" | "sinh" | "cosh" | "tanh" | "ⁿ√" | "logₙ" | "nCr" | "nPr" | "abs" => {
                let f: &'static str = match key { "sin⁻¹" => "asin", "cos⁻¹" => "acos", "tan⁻¹" => "atan", "sinh" => "sinh", "cosh" => "cosh", "tanh" => "tanh", "ⁿ√" => "root", "logₙ" => "logb", "nCr" => "nCr", "nPr" => "nPr", _ => "abs" };
                self.push_value_tok(Tok::Func(f));
            }
            "=" => self.equals(),
            _ => {}
        }
    }

    fn equals(&mut self) {
        if self.just_evaluated || self.tokens.is_empty() { return; }
        let has_x = self.tokens.contains(&Tok::X);
        let line0 = display_tokens(&self.tokens);
        if self.tokens.iter().any(|t| matches!(t, Tok::Cmp(_))) {
            let text = match self.ev().solve_ineq(&self.tokens) { Ok(s) => s, Err(e) => solve_error_text(&e).to_string() };
            self.s.history.insert(0, HistItem::new(line0.clone(), text.clone(), None));
            self.result = Some((line0, text)); self.tokens.clear(); self.just_evaluated = true; self.cursor = None;
            return;
        }
        if self.tokens.contains(&Tok::I) {
            let text = match crate::complex::eval(&self.tokens, self.s.degrees, self.s.ans) { Ok(z) => z.text(), Err(e) => expr::error_text(&e).to_string() };
            self.s.history.insert(0, HistItem::new(line0.clone(), text.clone(), None));
            self.result = Some((format!("{line0} ="), text)); self.tokens.clear(); self.just_evaluated = true; self.cursor = None;
            return;
        }
        if has_x && !self.tokens.contains(&Tok::Equals) {
            // first "=" in an equation inserts the equals sign; the next one solves
            self.tokens.push(Tok::Equals);
            return;
        }
        let line = display_tokens(&self.tokens);
        if has_x {
            let (text, value) = match self.ev().solve(&self.tokens) {
                Ok(r) if r.len() == 1 => (format!("x = {}", display_num(r[0])), Some(r[0])),
                Ok(r) => (r.iter().enumerate().map(|(i, v)| format!("x{} = {}", ["₁", "₂", "₃", "₄", "₅", "₆", "₇", "₈", "₉"].get(i).unwrap_or(&""), display_num(*v))).collect::<Vec<_>>().join(";  "), None),
                Err(e) => (solve_error_text(&e).to_string(), None),
            };
            self.s.history.insert(0, HistItem::new(line.clone(), text.clone(), value));
            self.result = Some((line, text));
            self.tokens = value.map(|v| vec![Tok::Num(plain(v))]).unwrap_or_default();
            if let Some(v) = value { self.s.ans = v; }
        } else {
            match self.ev().eval(&self.tokens) {
                Ok(v) => {
                    let text = display_num(v);
                    self.s.history.insert(0, HistItem::new(line.clone(), text.clone(), Some(v)));
                    self.result = Some((format!("{line} ="), text));
                    self.tokens = vec![Tok::Num(plain(v))];
                    self.s.ans = v;
                }
                Err(e) => { self.result = Some((format!("{line} ="), expr::error_text(&e).to_string())); self.tokens.clear(); }
            }
        }
        if self.s.history.len() > 500 { let mut n = 0; self.s.history.retain(|h| { n += 1; n <= 500 || h.pinned }); }
        self.just_evaluated = true;
        self.cursor = None;
    }

    pub fn memory(&mut self, key: &str) {
        let cur = self.current_value();
        match key {
            "M1" | "M2" | "M3" | "M4" | "M5" => { self.s.mem_slot = (self.s.mem_slot + 1) % 5; }
            "MC" => self.s.mem[self.s.mem_slot] = 0.0,
            "M+" => if let Some(v) = cur { self.s.mem[self.s.mem_slot] += v; self.just_evaluated = true; },
            "M−" => if let Some(v) = cur { self.s.mem[self.s.mem_slot] -= v; self.just_evaluated = true; },
            "MR" => {
                let m = self.s.mem[self.s.mem_slot];
                if self.just_evaluated || self.tokens.is_empty() { self.set_value(m); self.just_evaluated = false; }
                else if !is_value_end(self.tokens.last()) { self.tokens.push(Tok::Num(plain(m))); }
            }
            _ => {}
        }
    }

    pub fn paste(&mut self, text: &str) {
        if self.screen == Screen::Ask { self.ask_text = text.trim().chars().take(500).collect(); self.ask_out = None; return; }
        if self.restore_pending {
            self.restore_pending = false;
            match serde_json::from_str::<Saved>(text.trim()) {
                Ok(mut s) if !s.history.is_empty() || s.lang != 0 || s.theme != 0 => { s.rates = self.s.rates.take(); let sys = (self.system_dark, self.system_lang); *self = App::new(s); self.system_dark = sys.0; self.system_lang = sys.1; self.apply_lang(); self.screen = Screen::History; self.toast = Some(crate::i18n::t("Backup restored").into()); }
                _ => self.toast = Some(crate::i18n::t("No backup in clipboard. Tap Backup first, then copy it back.").into()),
            }
            return;
        }
        if let Some(raw) = expr::parse_pasted(text) {
            if self.just_evaluated || self.tokens.is_empty() { self.tokens = vec![Tok::Num(raw)]; self.just_evaluated = false; self.result = None; }
            else if !is_value_end(self.tokens.last()) { self.tokens.push(Tok::Num(raw)); }
            else { self.tokens.push(Tok::Op('×')); self.tokens.push(Tok::Num(raw)); }
            self.toast = Some(crate::i18n::t("Pasted").into());
        } else {
            self.toast = Some(crate::i18n::t("No number in clipboard").into());
        }
    }

    pub fn handle(&mut self, a: &Action) -> Vec<Effect> {
        let mut fx = vec![];
        let save_before = (self.s.mem[self.s.mem_slot], self.s.history.len(), self.s.theme, self.s.vibrate, self.s.degrees, self.s.sci);
        self.popup = matches!(a, Action::DisplayLongPress);
        match a {
            Action::Key(k) => {
                if self.s.vibrate { fx.push(Effect::Vibrate); }
                if self.s.sound { fx.push(Effect::Click); }
                if k.starts_with('M') { self.memory(k); }
                else if k == "DEG" || k == "RAD" { self.s.degrees = !self.s.degrees; }
                else {
                    self.press(k);
                    if k == "=" && self.s.speak { if let Some(v) = self.current_value() { fx.push(Effect::Speak(expr::words(v, crate::i18n::lang()).unwrap_or_else(|| expr::display_num(v)))); } }
                }
            }
            Action::Nav(s) => {
                self.screen = *s; self.scroll = 0.0;
                if *s == Screen::Ask && self.nano.is_empty() { fx.push(Effect::NanoStatus); }
                if *s == Screen::Graph {
                    let trig = self.tokens.iter().any(|t| matches!(t, Tok::Func("sin" | "cos" | "tan")));
                    self.graph_span = if trig && self.s.degrees { 360.0 } else { 10.0 };
                }
                if *s == Screen::Converter && self.s.conv_cat == CURRENCY_CAT { fx.push(Effect::FetchRates); }
            }
            Action::Back => {
                self.scroll = 0.0;
                self.screen = match self.screen { Screen::PickUnit { .. } => Screen::Converter, Screen::Gallery => Screen::Photo, Screen::Tool | Screen::Vat => Screen::Tools, _ => Screen::Calc };
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
            Action::ClearHistory => self.s.history.retain(|h| h.pinned),
            Action::Pin(i) => { if let Some(h) = self.s.history.get_mut(*i) { h.pinned = !h.pinned; } fx.push(Effect::Save); }
            Action::DeleteHistory(i) => { if *i < self.s.history.len() { self.s.history.remove(*i); self.toast = Some(crate::i18n::t("Deleted").into()); } fx.push(Effect::Save); }
            Action::Swipe(right, on) => match (self.screen, &**on) {
                (Screen::History, Action::LoadHistory(i)) => { let i = *i; if i < self.s.history.len() { self.s.history.remove(i); self.toast = Some(crate::i18n::t("Deleted").into()); fx.push(Effect::Save); } }
                (Screen::Calc, _) => { self.press(if *right { "↶" } else { "⌫" }); if self.s.vibrate { fx.push(Effect::Vibrate); } }
                _ => {}
            },
            Action::StartText(k, i) => { self.text_target = Some((*k, *i)); fx.push(Effect::Keyboard(true)); }
            Action::EndText => { self.text_target = None; fx.push(Effect::Keyboard(false)); fx.push(Effect::Save); }
            Action::TextKey(c) => match self.text_target {
                Some((0, _)) => self.search.push(*c),
                Some((2, _)) => if self.ask_text.chars().count() < 500 { self.ask_text.push(*c) },
                Some((1, i)) => if let Some(h) = self.s.history.get_mut(i) { if h.note.chars().count() < 200 { h.note.push(*c); } },
                _ => {}
            },
            Action::TextBack => match self.text_target {
                Some((0, _)) => { self.search.pop(); }
                Some((2, _)) => { self.ask_text.pop(); }
                Some((1, i)) => if let Some(h) = self.s.history.get_mut(i) { h.note.pop(); },
                _ => {}
            },
            Action::HwKey(c) => {
                // hardware keyboard on the calculator screen
                let k: Option<&str> = match c {
                    '0'..='9' => None, '+' => Some("+"), '-' => Some("−"), '*' => Some("×"), '/' => Some("÷"), '^' => Some("xʸ"), '(' => Some("("), ')' => Some(")"),
                    '.' | ',' => Some(","), '=' | '\n' => Some("="), '%' => Some("%"), '!' => Some("n!"), 'x' => Some("x"), '\u{8}' => Some("⌫"), '\u{1b}' => Some("C"), _ => Option::None,
                };
                if c.is_ascii_digit() { let s = c.to_string(); self.press(&s); } else if let Some(k) = k { self.press(k); }
            }
            Action::ToggleSci => self.s.sci = !self.s.sci,
            Action::CycleTheme => self.s.theme = (self.s.theme + 1) % 3,
            Action::CycleLang => { let n = crate::i18n::LANGS.len() as i32; self.s.lang = if self.s.lang + 1 >= n { -1 } else { self.s.lang + 1 }; self.apply_lang(); }
            Action::ToggleVibrate => self.s.vibrate = !self.s.vibrate,
            Action::ToggleAngle => self.s.degrees = !self.s.degrees,
            Action::CheckUpdate => { fx.push(Effect::CheckUpdate); self.toast = Some(crate::i18n::t("Checking for updates…").into()); }
            Action::UpdateBanner => fx.push(Effect::UpdateTap),
            Action::Copy => {
                let text = match self.current_value() { Some(v) => expr::clipboard_num(v), None => self.result.as_ref().map(|r| r.1.clone()).unwrap_or_default() };
                if !text.is_empty() { fx.push(Effect::Copy(text)); self.toast = Some(crate::i18n::t("Copied").into()); }
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
            Action::Set("gzoom+") => self.graph_span = (self.graph_span / 2.0).max(0.01),
            Action::Set("gzoom-") => self.graph_span = (self.graph_span * 2.0).min(1e6),
            Action::Set("fav") => { let c = self.s.conv_cat; if let Some(p) = self.s.fav_cats.iter().position(|x| *x == c) { self.s.fav_cats.remove(p); } else { self.s.fav_cats.push(c); } fx.push(Effect::Save); }
            Action::Set("ratehist") => { self.rate_hist = None; fx.push(Effect::FetchRateHistory); }
            Action::Set("export") => {
                let mut csv = String::from("expression;result;note\n");
                for h in &self.s.history { csv.push_str(&format!("\"{}\";\"{}\";\"{}\"\n", h.expr.replace('"', "'"), h.result.replace('"', "'"), h.note.replace('"', "'"))); }
                fx.push(Effect::SaveFile("calculator-history.csv".into(), csv.clone()));
                fx.push(Effect::Share(csv));
            }
            Action::Set("backup") => {
                let mut b = self.s.clone(); b.rates = None;
                if let Ok(js) = serde_json::to_string(&b) { fx.push(Effect::Copy(js.clone())); fx.push(Effect::SaveFile("calculator-backup.json".into(), js)); self.toast = Some(crate::i18n::t("Backup copied and saved to Downloads").into()); }
            }
            Action::Set("restore") => { self.restore_pending = true; fx.push(Effect::RequestPaste); }
            Action::Set("whatsnew") => self.whats_new = true,
            Action::PhotoPick(i) => {
                if let PhotoState::Done { bbox, text, tokens, others, .. } = &mut self.photo {
                    if *i < others.len() {
                        let (t2, b2) = others.remove(*i);
                        if let Some(b) = bbox.take() { others.insert(0, (text.clone(), b)); }
                        *tokens = crate::ocr::to_tokens(&t2); *text = t2; *bbox = Some(b2);
                    }
                }
            }
            Action::Set("photo_share") => { if let PhotoState::Done { tokens, .. } = &self.photo { let (ans, steps) = self.photo_answer(tokens); fx.push(Effect::Share(format!("{}\n{}\n{}", display_tokens(tokens), steps.join("\n"), ans))); } }
            Action::Set("photo_words") => { if let PhotoState::Done { text, .. } = &self.photo { self.ask_text = text.clone(); self.ask_out = None; self.screen = Screen::Ask; fx.push(Effect::Solve(self.ask_text.clone(), self.nano == "available")); if self.nano.is_empty() { fx.push(Effect::NanoStatus); } } }
            Action::Set("ask_voice") => { self.text_target = None; fx.push(Effect::Keyboard(false)); fx.push(Effect::Listen); }
            Action::Set("ask_paste") => fx.push(Effect::RequestPaste),
            Action::Set("ask_clear") => { self.ask_text.clear(); self.ask_out = None; self.ask_status.clear(); }
            Action::Set("ask_solve") => {
                self.text_target = None; fx.push(Effect::Keyboard(false));
                if self.ask_text.trim().is_empty() { self.ask_status = crate::i18n::t("Type or say a problem first.").into(); }
                else { self.ask_status = crate::i18n::t("Solving…").into(); self.ask_out = None; fx.push(Effect::Solve(self.ask_text.clone(), self.nano == "available")); }
            }
            Action::Set("ask_open") => { if let Some((e, _, _)) = &self.ask_out { if let Some(t) = expr::tokenize(e) { self.tokens = t; self.just_evaluated = false; self.result = None; self.screen = Screen::Calc; self.press("="); } } }
            Action::Set("closenew") => self.whats_new = false,
            Action::Set("crash_close") => self.crash = None,
            Action::Set("crash_copy") => { if let Some(c) = &self.crash { fx.push(Effect::Copy(format!("Calculator {}\n{}", version(), c))); self.toast = Some(crate::i18n::t("Copied").into()); } }
            Action::Set("crash_share") => { if let Some(c) = &self.crash { fx.push(Effect::Share(format!("Calculator {}\n{}", version(), c))); } }
            Action::Set("bug") => fx.push(Effect::ReportBug),
            Action::Set("floating") => fx.push(Effect::Floating),
            Action::Set("bgcheck") => { self.s.bg_check = !self.s.bg_check; fx.push(Effect::BgCheck(self.s.bg_check)); fx.push(Effect::Save); }
            Action::Set("speaknow") => { if let Some(v) = self.current_value() { fx.push(Effect::Speak(expr::words(v, crate::i18n::lang()).unwrap_or_else(|| expr::display_num(v)))); } }
            Action::Set("icon") => { self.cycle_setting("icon"); fx.push(Effect::SetIcon(self.s.icon)); fx.push(Effect::Save); }
            Action::Set(k) => { self.cycle_setting(k); fx.push(Effect::Save); }
            Action::OpenTool(i) => {
                self.tool = *i; self.tool_mode = 0; self.list.clear(); self.focus = 0;
                self.fields = vec![String::new(); crate::tools::TOOLS[*i].fields.len()];
                // sensible prefills: calculator value into the first field, today into dates
                if let Some(v) = self.current_value().filter(|v| *v != 0.0) { if *i != crate::tools::T_DATES && *i != crate::tools::T_RANDOM { self.fields[0] = plain(v); } }
                if *i == crate::tools::T_DATES { let (y, m, d) = crate::tools::civil_from_days(crate::tools::today_days()); self.fields = vec![y.to_string(), m.to_string(), d.to_string(), y.to_string(), m.to_string(), d.to_string()]; }
                if *i == crate::tools::T_RANDOM { self.fields = vec!["1".into(), "100".into()]; }
                if *i == 4 { self.fields[1] = "0".into(); }
                if *i == 5 { self.fields[3] = "1".into(); }
                self.screen = Screen::Tool; self.scroll = 0.0;
            }
            Action::ToolMode(m) => { self.tool_mode = *m; let vis = crate::tools::visible_fields(self.tool, *m); if !vis.contains(&self.focus) { self.focus = vis[0]; } self.reroll(); }
            Action::Focus(f) => self.focus = *f,
            Action::FormKey(k) => { if self.s.vibrate { fx.push(Effect::Vibrate); } self.form_key(k); }
            Action::GalleryPick(id) => { self.screen = Screen::Photo; self.photo = PhotoState::Working; fx.push(Effect::LoadGallery(*id)); }
        }
        let after = (self.s.mem[self.s.mem_slot], self.s.history.len(), self.s.theme, self.s.vibrate, self.s.degrees, self.s.sci);
        if after != save_before && !fx.contains(&Effect::Save) { fx.push(Effect::Save); }
        fx
    }

    fn reroll(&mut self) { let mut x = self.seed ^ (self.seed << 13); x ^= x >> 7; x ^= x << 17; self.seed = x; }

    pub fn cycle_setting(&mut self, k: &str) {
        let s = &mut self.s;
        match k {
            "decimal" => s.decimal = (s.decimal + 1) % 2,
            "grouping" => s.grouping = (s.grouping + 1) % 4,
            "fixed" => s.fixed = match s.fixed { -1 => 0, 0 => 2, 2 => 4, 4 => 6, 6 => 10, _ => -1 },
            "words" => s.words = !s.words,
            "accent" => s.accent = (s.accent + 1) % 7,
            "contrast" => s.contrast = !s.contrast,
            "keys" => s.key_size = (s.key_size + 1) % 3,
            "font" => s.font_size = (s.font_size + 1) % 4,
            "onehand" => s.one_hand = (s.one_hand + 1) % 3,
            "anim" => s.animations = !s.animations,
            "icon" => s.icon = (s.icon + 1) % 3,
            "sound" => s.sound = !s.sound,
            "vib" => s.vib_ms = match s.vib_ms { 0..=10 => 20, 11..=20 => 40, 21..=40 => 70, _ => 10 },
            "speak" => s.speak = !s.speak,
            "start" => s.start_screen = (s.start_screen + 1) % 4,
            "beta" => s.beta = !s.beta,
            _ => {}
        }
        self.apply_format();
        self.apply_look();
    }

    /// Numeric values of the tool fields (NaN when empty or invalid).
    pub fn field_values(&self) -> Vec<f64> { self.fields.iter().map(|f| f.replace(',', ".").parse::<f64>().unwrap_or(f64::NAN)).collect() }

    fn form_key(&mut self, k: &str) {
        let vis = crate::tools::visible_fields(self.tool, self.tool_mode);
        let hex = self.tool == crate::tools::T_BASES;
        let Some(f) = self.fields.get_mut(self.focus) else { return };
        match k {
            "⌫" => { f.pop(); }
            "C" => { f.clear(); }
            "," => { if !hex && !f.contains('.') { if f.is_empty() || f == "-" { f.push('0'); } f.push('.'); } }
            "±" => { if f.starts_with('-') { f.remove(0); } else { f.insert(0, '-'); } }
            "next" => { let p = vis.iter().position(|v| *v == self.focus).unwrap_or(0); self.focus = vis[(p + 1) % vis.len()]; }
            "add" => {
                let v = f.replace(',', ".").parse::<f64>();
                if let Ok(v) = v { if self.list.len() < 200 { self.list.push(v); } f.clear(); }
            }
            "undo" => { self.list.pop(); }
            "roll" => self.reroll(),
            "C_" => { f.push('C'); }
            d => { if f.len() < 18 { f.push_str(d); } }
        }
    }

    /// Processes a decoded photo (runs OCR; call from a worker thread with a clone of the image).
    pub fn photo_result(img: image::RgbImage) -> PhotoState {
        match crate::ocr::recognize_lines(&img) {
            Ok(lines) => {
                // a "-" at the very start of a line that touches the photo edge is usually the paper edge or a shadow
                let edge = img.width() as f32 * 0.03;
                let texts: Vec<String> = lines.iter().map(|l| { let t = l.text.trim_start(); if l.bbox[0] <= edge && t.starts_with('-') && t[1..].trim_start().starts_with(|c: char| c.is_ascii_digit()) { t[1..].trim_start().to_string() } else { l.text.clone() } }).collect();
                // every other line that also looks like a problem can be picked instead
                let mut others: Vec<(String, [f32; 4])> = vec![];
                match crate::ocr::best_index(&texts) {
                    Some(i) => {
                        let tokens = crate::ocr::to_tokens(&texts[i]);
                        if tokens.is_empty() { return PhotoState::Failed(crate::i18n::t("No problem found in the photo.").into()); }
                        for (j, t) in texts.iter().enumerate() { if j != i && t.chars().filter(|c| c.is_ascii_digit()).count() >= 2 && t.chars().any(|c| "+-−×÷*/=:".contains(c)) && !crate::ocr::to_tokens(t).is_empty() && others.len() < 6 { others.push((t.clone(), lines[j].bbox)); } }
                        PhotoState::Done { bbox: Some(lines[i].bbox), text: texts[i].clone(), tokens, img: thumb(&img, 900), others }
                    }
                    None => {
                        // a problem written in words: read the whole text
                        let all = texts.join(" ");
                        match expr::text_to_expr(&all).and_then(|e| expr::tokenize(&e)) {
                            Some(tokens) => PhotoState::Done { bbox: None, text: all, tokens, img: thumb(&img, 900), others },
                            None => PhotoState::Failed(crate::i18n::t("No problem found in the photo. Move closer so the numbers are sharp.").into()),
                        }
                    }
                }
            }
            Err(e) => PhotoState::Failed(crate::i18n::tf("Recognition failed: {}", &[&e])),
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
                    steps.push(crate::i18n::tf("Move terms: {}x = {}", &[&display_num(b), &display_num(-c)]));
                    steps.push(crate::i18n::tf("Divide by {}: x = {} ÷ {}", &[&display_num(b), &display_num(-c), &display_num(b)]));
                } else if a.abs() > 1e-12 {
                    steps.push(crate::i18n::tf("Quadratic equation: a = {}, b = {}, c = {}", &[&display_num(a), &display_num(b), &display_num(c)]));
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
        SolveError::NoSolution => crate::i18n::t("No solutions"),
        SolveError::AllX => crate::i18n::t("Any x works"),
        SolveError::NoRealSolution => crate::i18n::t("No real solutions"),
        SolveError::Calc(c) => expr::error_text(c),
    }
}

// ---------------- theme ----------------

pub struct Theme { pub bg: Rgb, pub key: Rgb, pub key_op: Rgb, pub key_eq: Rgb, pub key_eq_text: Rgb, pub key_text: Rgb, pub text: Rgb, pub dim: Rgb, pub chip: Rgb, pub chip_on: Rgb, pub chip_on_text: Rgb, pub pressed: Rgb, pub line: Rgb, pub shadow: Rgb }

pub static LOOK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub static CONTRAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static FONT_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1000);
pub fn font_scale() -> f32 { FONT_SCALE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 1000.0 }

pub fn theme(dark: bool) -> Theme {
    let mut th = base_theme(dark);
    if CONTRAST.load(std::sync::atomic::Ordering::Relaxed) {
        th = if dark {
            Theme { bg: [0, 0, 0], key: [0x1A, 0x1A, 0x1A], key_op: [0x33, 0x33, 0x00], key_eq: [0xFF, 0xE0, 0x00], key_eq_text: [0, 0, 0], key_text: [0xFF, 0xFF, 0xFF], text: [0xFF, 0xFF, 0xFF], dim: [0xFF, 0xFF, 0xFF],
                chip: [0x1A, 0x1A, 0x1A], chip_on: [0xFF, 0xE0, 0x00], chip_on_text: [0, 0, 0], pressed: [0x66, 0x66, 0x66], line: [0xFF, 0xFF, 0xFF], shadow: [0, 0, 0] }
        } else {
            Theme { bg: [0xFF, 0xFF, 0xFF], key: [0xFF, 0xFF, 0xFF], key_op: [0xE6, 0xE6, 0xE6], key_eq: [0, 0, 0], key_eq_text: [0xFF, 0xFF, 0xFF], key_text: [0, 0, 0], text: [0, 0, 0], dim: [0, 0, 0],
                chip: [0xE6, 0xE6, 0xE6], chip_on: [0, 0, 0], chip_on_text: [0xFF, 0xFF, 0xFF], pressed: [0x99, 0x99, 0x99], line: [0, 0, 0], shadow: [0, 0, 0] }
        };
        return th;
    }
    let v = LOOK.load(std::sync::atomic::Ordering::Relaxed);
    if v != 0 {
        let c: Rgb = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        let mix = |a: Rgb, b: Rgb, f: f32| -> Rgb { [0, 1, 2].map(|i| (a[i] as f32 * (1.0 - f) + b[i] as f32 * f) as u8) };
        th.key_eq = c; th.key_eq_text = [0xFF, 0xFF, 0xFF];
        th.chip_on = c; th.chip_on_text = [0xFF, 0xFF, 0xFF];
        th.key_op = mix(th.key_op, c, if dark { 0.30 } else { 0.22 });
    }
    th
}

fn base_theme(dark: bool) -> Theme {
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
    /// Polylines in screen coordinates (graphs); color index 0 = text, 1 = accent, 2 = dim grid.
    Plot { r: Rect, lines: Vec<(u8, Vec<(f32, f32)>)> },
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
                let kp_h = if self.screen == Screen::Tool { (h * 0.34).min(300.0 * d) } else { 0.0 };
                let area = Rect::new(0.0, body, w, h - body - kp_h);
                if kp_h > 0.0 { self.build_form_keys(&mut out, Rect::new(0.0, h - kp_h, w, kp_h), d, &t); }
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
        if self.whats_new {
            let r = Rect::new(16.0 * d, h * 0.18, w - 32.0 * d, h * 0.62);
            out.push(W::Fill { r: Rect::new(0.0, 0.0, w, h), c: t.shadow, radius: 0.0 });
            out.push(W::Button { r: Rect::new(0.0, 0.0, w, h), label: String::new(), px: 0.0, kind: Kind::Invisible, action: Action::Set("closenew") });
            out.push(W::Fill { r, c: t.bg, radius: 14.0 * d });
            out.push(W::Text { r: Rect::new(r.x + 16.0 * d, r.y + 10.0 * d, r.w - 32.0 * d, 36.0 * d), text: crate::i18n::tf("What's new in {}", &[&version()]), px: 19.0 * d, bold: true, c: t.text, align: 0, wrap: false });
            out.push(W::Text { r: Rect::new(r.x + 16.0 * d, r.y + 50.0 * d, r.w - 32.0 * d, r.h - 120.0 * d), text: crate::i18n::t(WHATS_NEW).into(), px: 14.0 * d, bold: false, c: t.text, align: 0, wrap: true });
            out.push(W::Button { r: Rect::new(r.x + 16.0 * d, r.bottom() - 60.0 * d, r.w - 32.0 * d, 46.0 * d), label: crate::i18n::t("OK").into(), px: 16.0 * d, kind: Kind::ChipOn, action: Action::Set("closenew") });
        }
        if let Some(c) = &self.crash {
            let r = Rect::new(16.0 * d, h * 0.12, w - 32.0 * d, h * 0.72);
            out.push(W::Fill { r: Rect::new(0.0, 0.0, w, h), c: t.shadow, radius: 0.0 });
            out.push(W::Button { r: Rect::new(0.0, 0.0, w, h), label: String::new(), px: 0.0, kind: Kind::Invisible, action: Action::None });
            out.push(W::Fill { r, c: t.bg, radius: 14.0 * d });
            out.push(W::Text { r: Rect::new(r.x + 16.0 * d, r.y + 10.0 * d, r.w - 32.0 * d, 56.0 * d), text: crate::i18n::t("The app closed because of an error").into(), px: 17.0 * d, bold: true, c: t.text, align: 0, wrap: true });
            out.push(W::Text { r: Rect::new(r.x + 16.0 * d, r.y + 66.0 * d, r.w - 32.0 * d, 40.0 * d), text: crate::i18n::t("Please copy or share this text and send it to the developer.").into(), px: 13.0 * d, bold: false, c: t.text, align: 0, wrap: true });
            let mut shown: String = c.replace('\t', " ").chars().take(360).collect(); if c.chars().count() > 360 { shown.push_str(" …"); }
            out.push(W::Text { r: Rect::new(r.x + 16.0 * d, r.y + 112.0 * d, r.w - 32.0 * d, r.h - 190.0 * d), text: shown, px: 11.0 * d, bold: false, c: t.text, align: 0, wrap: true });
            let bw = (r.w - 32.0 * d - 16.0 * d) / 3.0;
            let by = r.bottom() - 60.0 * d;
            out.push(W::Button { r: Rect::new(r.x + 16.0 * d, by, bw, 46.0 * d), label: crate::i18n::t("Copy").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::Set("crash_copy") });
            out.push(W::Button { r: Rect::new(r.x + 24.0 * d + bw, by, bw, 46.0 * d), label: crate::i18n::t("Share").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::Set("crash_share") });
            out.push(W::Button { r: Rect::new(r.x + 32.0 * d + 2.0 * bw, by, bw, 46.0 * d), label: crate::i18n::t("OK").into(), px: 14.0 * d, kind: Kind::ChipOn, action: Action::Set("crash_close") });
        }
        Frame { widgets: out, scroll_area, page }
    }

    fn build_calc(&self, out: &mut Vec<W>, w: f32, h: f32, d: f32, t: &Theme) {
        let landscape = w > h;
        let pad = 12.0 * d;
        // top bar
        let bar_h = 40.0 * d;
        let chips: [(&str, Action, bool); 6] = [
            (crate::i18n::t("History"), Action::Nav(Screen::History), false),
            (crate::i18n::t("Scientific"), Action::ToggleSci, self.s.sci || landscape),
            (crate::i18n::t("Converter"), Action::Nav(Screen::Converter), false),
            (crate::i18n::t("Tools"), Action::Nav(Screen::Tools), false),
            (crate::i18n::t("Photo"), Action::Nav(Screen::Photo), false),
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
        let kf = [1.0, 0.8, 1.2][self.s.key_size as usize % 3];
        let disp_h = (if landscape { avail * 0.28 } else if sci { avail * 0.22 } else { avail * 0.26 }) * kf;
        let disp = Rect::new(pad, top, inner_w, disp_h);
        self.build_display(out, disp, d, t);
        out.push(W::Button { r: disp, label: String::new(), px: 0.0, kind: Kind::Invisible, action: if self.popup { Action::ClosePopup } else { Action::Copy } });
        // memory row
        let mem = Rect::new(pad, disp.bottom(), inner_w, mem_h);
        let slot = format!("M{}", self.s.mem_slot + 1);
        let mkeys = [slot.as_str(), "MC", "M+", "M−", "MR"];
        let mw = mem.w / 5.0;
        for (i, k) in mkeys.iter().enumerate() {
            out.push(W::Button { r: Rect::new(mem.x + i as f32 * mw, mem.y, mw, mem.h), label: k.to_string(), px: 15.0 * d, kind: Kind::Flat, action: Action::Key(k.to_string()) });
        }
        let grid_top = mem.bottom() + 2.0 * d;
        let grid_h = h - pad - grid_top;
        let basic: [&str; 20] = ["C", "⌫", "÷", "×", "7", "8", "9", "−", "4", "5", "6", "+", "1", "2", "3", "=", "0", ",", "±", "%"];
        let deg = if self.s.degrees { "DEG" } else { "RAD" };
        let scik: Vec<&str> = if self.sci_page == 0 {
            vec!["2nd", "(", ")", "◀", "▶", "x²", "xʸ", "√", "π", "e", "1/x", "sin", "cos", "tan", "n!", "ln", "log", "x", "Ans", deg]
        } else {
            vec!["2nd", "sin⁻¹", "cos⁻¹", "tan⁻¹", "mod", "sinh", "cosh", "tanh", "ⁿ√", "logₙ", "nCr", "nPr", ";", "EE", "a/b", "i", "<", ">", "↶", "↷"]
        };
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
            place(out, &scik, Rect::new(pad, grid_top, half, grid_h), 4, 5, 15.0 * d);
            place(out, &basic, Rect::new(pad + half, grid_top, half, grid_h), 4, 5, 18.0 * d);
        } else {
            // tablets: keep keys a sensible size; one-hand mode: narrower keypad on one side
            let mut kw = inner_w.min(560.0 * d);
            if self.s.one_hand > 0 { kw = kw * 0.8; }
            let kx = match self.s.one_hand { 1 => pad + inner_w - kw, 2 => pad, _ => pad + (inner_w - kw) / 2.0 };
            let (gt, gh) = if self.s.one_hand > 0 { let gh = grid_h * 0.85; (grid_top + grid_h - gh, gh) } else { (grid_top, grid_h) };
            if sci {
                let sci_h = gh * 5.0 / 10.0 * 0.8;
                place(out, &scik, Rect::new(kx, gt, kw, sci_h), 4, 5, 16.0 * d);
                place(out, &basic, Rect::new(kx, gt + sci_h, kw, gh - sci_h), 4, 5, 22.0 * d);
            } else {
                place(out, &basic, Rect::new(kx, gt, kw, gh), 4, 5, 22.0 * d);
            }
        }
        if self.popup {
            let pw = 240.0 * d;
            let r = Rect::new(disp.right() - pw, disp.y + 8.0 * d, pw, 44.0 * d);
            out.push(W::Fill { r: Rect { y: r.y + 2.0 * d, ..r }, c: t.shadow, radius: 10.0 * d });
            out.push(W::Fill { r, c: t.chip_on, radius: 10.0 * d });
            out.push(W::Button { r: Rect { w: pw / 2.0, ..r }, label: crate::i18n::t("Copy").into(), px: 15.0 * d, kind: Kind::ChipOn, action: Action::Copy });
            out.push(W::Button { r: Rect { x: r.x + pw / 2.0, w: pw / 2.0, ..r }, label: crate::i18n::t("Paste").into(), px: 15.0 * d, kind: Kind::ChipOn, action: Action::Paste });
        }
    }

    fn build_display(&self, out: &mut Vec<W>, r: Rect, d: f32, t: &Theme) {
        let inner = r.inset(8.0 * d, 4.0 * d);
        // indicators
        let mut ind = vec![];
        if self.s.mem[self.s.mem_slot] != 0.0 { ind.push(format!("M{}", self.s.mem_slot + 1)); }
        if self.sci_page == 1 { ind.push("2nd".into()); }
        if self.s.sci { ind.push(if self.s.degrees { "DEG" } else { "RAD" }.to_string()); }
        if !ind.is_empty() {
            out.push(W::Text { r: Rect::new(inner.x, inner.y, inner.w, 18.0 * d), text: ind.join("  "), px: 12.0 * d, bold: true, c: t.dim, align: 0, wrap: false });
        }
        let (small, big, big_bold) = match &self.result {
            Some((line, res)) => {
                let mut res = res.clone();
                if self.show_frac { if let Some(f) = self.s.history.first().and_then(|h| h.value).and_then(expr::fraction_text) { res = f; } }
                (line.clone(), res, true)
            }
            None => {
                let mut e = match self.cursor {
                    Some(c) if c <= self.tokens.len() => format!("{}|{}", display_tokens(&self.tokens[..c]), display_tokens(&self.tokens[c..])),
                    _ => display_tokens(&self.tokens),
                };
                let open = self.open_parens();
                if open > 0 && !e.is_empty() { e.push_str(&")".repeat(open as usize)); }
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
        if self.s.words && self.result.is_some() {
            if let Some(w) = self.s.history.first().and_then(|h| h.value).and_then(|v| expr::words(v, crate::i18n::lang())) {
                out.push(W::Text { r: Rect::new(inner.x, inner.bottom() - 14.0 * d, inner.w, 14.0 * d), text: w, px: 12.0 * d, bold: false, c: t.dim, align: 2, wrap: false });
            }
        }
    }

    fn build_form_keys(&self, out: &mut Vec<W>, area: Rect, d: f32, t: &Theme) {
        out.push(W::Fill { r: area, c: t.bg, radius: 0.0 });
        out.push(W::Fill { r: Rect::new(0.0, area.y, area.w, 1.0 * d), c: t.line, radius: 0.0 });
        let def = &crate::tools::TOOLS[self.tool];
        let hex = self.tool == crate::tools::T_BASES && self.tool_mode == 3;
        let last: &'static str = if def.list { "add" } else { "next" };
        let mut keys: Vec<&'static str> = vec!["7", "8", "9", "⌫", "4", "5", "6", "C", "1", "2", "3", "±", "0", ",", if def.list { "undo" } else { "roll" }, last];
        if self.tool != crate::tools::T_RANDOM && !def.list { keys[14] = "next"; keys[15] = "done"; }
        if hex { keys = vec!["7", "8", "9", "⌫", "4", "5", "6", "C", "1", "2", "3", "A", "0", "B", "C_", "D", "E", "F", "next", "done"]; }
        let cols = 4; let rows = (keys.len() + cols - 1) / cols;
        let inner = area.inset(8.0 * d, 6.0 * d);
        let (cw, rh) = (inner.w / cols as f32, inner.h / rows as f32);
        for (i, k) in keys.iter().enumerate() {
            let r = Rect::new(inner.x + (i % cols) as f32 * cw + 3.0 * d, inner.y + (i / cols) as f32 * rh + 3.0 * d, cw - 6.0 * d, rh - 6.0 * d);
            let (label, act): (String, &'static str) = match *k {
                "next" => (crate::i18n::t("Next").into(), "next"), "add" => (crate::i18n::t("Add").into(), "add"), "undo" => (crate::i18n::t("Remove last").into(), "undo"),
                "roll" => (crate::i18n::t("Again").into(), "roll"), "done" => (crate::i18n::t("Done").into(), "done"), "C_" => ("C".into(), "C_"), k => (k.to_string(), k),
            };
            let kind = match act { "add" | "done" | "roll" => Kind::Eq, "⌫" | "C" | "±" | "next" | "undo" => Kind::Op, _ => Kind::Digit };
            let action = if act == "done" { Action::ClosePopup } else if act == "C_" { Action::FormKey("C_") } else { Action::FormKey(act) };
            out.push(W::Button { r, label, px: 19.0 * d, kind, action });
        }
    }

    /// Expression to graph: the calculator expression with x (an equation is graphed as left − right).
    pub fn graph_tokens(&self) -> Vec<Tok> {
        let src = if self.tokens.contains(&Tok::X) { self.tokens.clone() } else { self.s.history.iter().find(|h| h.expr.contains('x')).map(|_| vec![]).unwrap_or_default() };
        if let Some(p) = src.iter().position(|t| matches!(t, Tok::Equals | Tok::Cmp(_))) {
            let mut v = vec![Tok::LParen]; v.extend_from_slice(&src[..p]); v.push(Tok::RParen); v.push(Tok::Op('−')); v.push(Tok::LParen); v.extend_from_slice(&src[p + 1..]); v.push(Tok::RParen); v
        } else { src }
    }

    /// Returns the y where the page body starts.
    fn build_header(&self, out: &mut Vec<W>, w: f32, d: f32, t: &Theme) -> f32 {
        let title = match self.screen {
            Screen::History => crate::i18n::t("History"), Screen::Vat => crate::i18n::t("VAT and tips"), Screen::Converter => crate::i18n::t("Converter"),
            Screen::PickUnit { .. } => crate::i18n::t("Choose a unit"), Screen::Settings => crate::i18n::t("Settings"), Screen::Photo => crate::i18n::t("Photo problem"),
            Screen::Gallery => crate::i18n::t("Choose a photo"), Screen::Calc => "",
            Screen::Tools => crate::i18n::t("Tools"), Screen::Tool => crate::i18n::t(crate::tools::TOOLS[self.tool].name), Screen::Graph => crate::i18n::t("Graph"), Screen::Ask => crate::i18n::t("Word problem"),
        };
        let h = 56.0 * d;
        out.push(W::Button { r: Rect::new(4.0 * d, 4.0 * d, 48.0 * d, 48.0 * d), label: "‹".into(), px: 30.0 * d, kind: Kind::Flat, action: Action::Back });
        out.push(W::Text { r: Rect::new(56.0 * d, 0.0, w - 170.0 * d, h), text: title.into(), px: 19.0 * d, bold: true, c: t.text, align: 0, wrap: false });
        if self.screen == Screen::History && !self.s.history.is_empty() {
            out.push(W::Button { r: Rect::new(w - 110.0 * d, 10.0 * d, 98.0 * d, 36.0 * d), label: crate::i18n::t("Clear").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::ClearHistory });
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
                // search + data buttons
                let searching = self.text_target.map(|t| t.0 == 0).unwrap_or(false);
                let sr = Rect::new(x, y, w, 48.0 * d);
                out.push(W::Button { r: sr, label: String::new(), px: 0.0, kind: if searching { Kind::ChipOn } else { Kind::Row }, action: if searching { Action::EndText } else { Action::StartText(0, 0) } });
                let stext = if self.search.is_empty() && !searching { format!("{}…", crate::i18n::t("Search")) } else { format!("{}{}", self.search, if searching { "|" } else { "" }) };
                out.push(W::Text { r: sr.inset(12.0 * d, 0.0), text: stext, px: 15.0 * d, bold: false, c: if searching { t.chip_on_text } else { t.dim }, align: 0, wrap: false });
                y += 56.0 * d;
                chips(out, &mut y, &[(crate::i18n::t("Export").to_string(), Action::Set("export"), false), (crate::i18n::t("Backup").to_string(), Action::Set("backup"), false), (crate::i18n::t("Restore").to_string(), Action::Set("restore"), false)]);
                if self.s.history.is_empty() { para(out, &mut y, crate::i18n::t("History is empty. Your calculations will appear here."), 15.0 * d); }
                let q = self.search.to_lowercase();
                let mut idx: Vec<usize> = (0..self.s.history.len()).filter(|&i| { let h = &self.s.history[i]; q.is_empty() || h.expr.to_lowercase().contains(&q) || h.result.to_lowercase().contains(&q) || h.note.to_lowercase().contains(&q) || h.expr.replace(|c: char| !c.is_ascii_digit(), "").contains(&q) }).collect();
                idx.sort_by_key(|&i| !self.s.history[i].pinned);
                for i in idx {
                    let hi = &self.s.history[i];
                    let editing = self.text_target == Some((1, i));
                    let has_note = !hi.note.is_empty() || editing;
                    let rh = if has_note { 88.0 } else { 64.0 } * d;
                    let r = Rect::new(x, y, w, rh);
                    out.push(W::Button { r, label: String::new(), px: 0.0, kind: Kind::Row, action: Action::LoadHistory(i) });
                    let tag = if hi.photo { "◉ " } else { "" };
                    out.push(W::Text { r: Rect::new(r.x + 96.0 * d, r.y + 4.0 * d, r.w - 108.0 * d, 26.0 * d), text: format!("{tag}{}", hi.expr), px: 14.0 * d, bold: false, c: t.dim, align: 2, wrap: false });
                    out.push(W::Text { r: Rect::new(r.x + 96.0 * d, r.y + 28.0 * d, r.w - 108.0 * d, 32.0 * d), text: hi.result.clone(), px: 20.0 * d, bold: true, c: t.text, align: 2, wrap: false });
                    if has_note { out.push(W::Text { r: Rect::new(r.x + 12.0 * d, r.y + 60.0 * d, r.w - 24.0 * d, 24.0 * d), text: format!("✎ {}{}", hi.note, if editing { "|" } else { "" }), px: 13.0 * d, bold: false, c: t.dim, align: 0, wrap: false }); }
                    out.push(W::Button { r: Rect::new(r.x + 4.0 * d, r.y + 8.0 * d, 42.0 * d, 44.0 * d), label: if hi.pinned { "★".into() } else { "☆".into() }, px: 20.0 * d, kind: Kind::Flat, action: Action::Pin(i) });
                    out.push(W::Button { r: Rect::new(r.x + 48.0 * d, r.y + 8.0 * d, 42.0 * d, 44.0 * d), label: "✎".into(), px: 18.0 * d, kind: Kind::Flat, action: if editing { Action::EndText } else { Action::StartText(1, i) } });
                    y += rh + 4.0 * d;
                }
                if !self.s.history.is_empty() { para(out, &mut y, crate::i18n::t("Swipe an entry sideways to delete it. ★ keeps it at the top."), 12.0 * d); }
            }
            Screen::Vat => {
                let v = value.unwrap_or(0.0);
                row(out, &mut y, crate::i18n::t("Amount (from calculator)"), &display_num(v), Action::None);
                heading(out, &mut y, "PVM 21 %");
                row(out, &mut y, "Su PVM (+21 %)", &display_num(v * 1.21), Action::UseValue(v * 1.21));
                row(out, &mut y, crate::i18n::t("Without VAT (amount incl. VAT)"), &display_num(v / 1.21), Action::UseValue(v / 1.21));
                row(out, &mut y, crate::i18n::t("VAT from amount"), &display_num(v * 0.21), Action::UseValue(v * 0.21));
                row(out, &mut y, crate::i18n::t("VAT inside amount incl. VAT"), &display_num(v - v / 1.21), Action::UseValue(v - v / 1.21));
                heading(out, &mut y, crate::i18n::t("Tip"));
                let tips = [5.0, 10.0, 15.0, 20.0];
                chips(out, &mut y, &tips.iter().enumerate().map(|(i, p)| (format!("{} %", *p as i32), Action::Tip(i), self.s.tip == i)).collect::<Vec<_>>());
                let p = tips[self.s.tip.min(3)] / 100.0;
                row(out, &mut y, crate::i18n::t("Tip"), &display_num(v * p), Action::UseValue(v * p));
                row(out, &mut y, crate::i18n::t("Total"), &display_num(v * (1.0 + p)), Action::UseValue(v * (1.0 + p)));
                heading(out, &mut y, crate::i18n::t("Split between people"));
                chips(out, &mut y, &(1..=6).map(|n| (n.to_string(), Action::People(n), self.s.people == n)).collect::<Vec<_>>());
                let each = v * (1.0 + p) / self.s.people.max(1) as f64;
                row(out, &mut y, crate::i18n::t("Per person"), &display_num(each), Action::UseValue(each));
                para(out, &mut y, crate::i18n::t("Tap a row to move the result to the calculator."), 13.0 * d);
            }
            Screen::Converter => {
                let c = self.s.conv_cat;
                let mut order: Vec<usize> = self.s.fav_cats.iter().cloned().filter(|i| *i < CATEGORIES.len()).collect();
                for i in 0..CATEGORIES.len() { if !order.contains(&i) { order.push(i); } }
                chips(out, &mut y, &order.iter().map(|&i| (format!("{}{}", if self.s.fav_cats.contains(&i) { "★ " } else { "" }, crate::i18n::t(CATEGORIES[i].name)), Action::ConvCat(i), i == c)).collect::<Vec<_>>());
                let fav = self.s.fav_cats.contains(&c);
                out.push(W::Button { r: Rect::new(x, y - 6.0 * d, 200.0 * d, 34.0 * d), label: if fav { format!("★ {}", crate::i18n::t("Favorite")) } else { format!("☆ {}", crate::i18n::t("Add to favorites")) }, px: 13.0 * d, kind: Kind::Flat, action: Action::Set("fav") });
                y += 36.0 * d;
                let names = convert::unit_names(c, &self.s.rates);
                let (fi, ti) = (self.s.conv_from[c].min(names.len().saturating_sub(1)), self.s.conv_to[c].min(names.len().saturating_sub(1)));
                let v = value.unwrap_or(1.0);
                row(out, &mut y, &crate::i18n::tf("From: {}  ▾", &[&names.get(fi).cloned().unwrap_or_default()]), &display_num(v), Action::Nav(Screen::PickUnit { from: true }));
                out.push(W::Button { r: Rect::new(x + w / 2.0 - 30.0 * d, y - 6.0 * d, 60.0 * d, 36.0 * d), label: "⇅".into(), px: 20.0 * d, kind: Kind::Chip, action: Action::SwapUnits });
                y += 34.0 * d;
                let res = convert::convert(c, fi, ti, v, &self.s.rates);
                row(out, &mut y, &crate::i18n::tf("To: {}  ▾", &[&names.get(ti).cloned().unwrap_or_default()]), &res.map(display_num).unwrap_or("—".into()), Action::Nav(Screen::PickUnit { from: false }));
                if let Some(r) = res { out.push(W::Button { r: Rect::new(x, y, w, 40.0 * d), label: crate::i18n::t("Move result to calculator").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::UseValue(r) }); y += 52.0 * d; }
                if c == CURRENCY_CAT {
                    let note = match &self.s.rates {
                        Some(r) => crate::i18n::tf("European Central Bank rates, {}. {}", &[&r.date, &self.rates_status]),
                        None => crate::i18n::tf("Rates not downloaded yet. Internet needed. {}", &[&self.rates_status]),
                    };
                    para(out, &mut y, &note, 13.0 * d);
                    out.push(W::Button { r: Rect::new(x, y, 160.0 * d, 36.0 * d), label: crate::i18n::t("Update rates").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::RefreshRates });
                    out.push(W::Button { r: Rect::new(x + 170.0 * d, y, 190.0 * d, 36.0 * d), label: crate::i18n::t("Rate history (90 days)").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::Set("ratehist") });
                    y += 48.0 * d;
                    let (fname, tname) = (names.get(fi).cloned().unwrap_or_default(), names.get(ti).cloned().unwrap_or_default());
                    if let Some((a, b, pts)) = &self.rate_hist {
                        if *a == fname && *b == tname && pts.len() > 1 {
                            let r = Rect::new(x, y, w, w * 0.55);
                            out.push(W::Fill { r, c: t.chip, radius: 8.0 * d });
                            let lo = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min); let hi = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                            let span = if hi > lo { hi - lo } else { 1.0 };
                            let line: Vec<(f32, f32)> = pts.iter().enumerate().map(|(i, p)| (r.x + 6.0 * d + (r.w - 12.0 * d) * i as f32 / (pts.len() - 1) as f32, r.y + 24.0 * d + (r.h - 48.0 * d) * (1.0 - ((p.1 - lo) / span) as f32))).collect();
                            out.push(W::Plot { r, lines: vec![(1, line)] });
                            out.push(W::Text { r: Rect::new(r.x + 6.0 * d, r.y + 2.0 * d, r.w - 12.0 * d, 20.0 * d), text: format!("1 {a} = {} {b}  (max)", display_num((hi * 1e4).round() / 1e4)), px: 11.0 * d, bold: false, c: t.dim, align: 0, wrap: false });
                            out.push(W::Text { r: Rect::new(r.x + 6.0 * d, r.bottom() - 22.0 * d, r.w - 12.0 * d, 20.0 * d), text: format!("{} … {}   min {}", pts[0].0, pts[pts.len() - 1].0, display_num((lo * 1e4).round() / 1e4)), px: 11.0 * d, bold: false, c: t.dim, align: 0, wrap: false });
                            y += r.h + 10.0 * d;
                        }
                    }
                }
                para(out, &mut y, crate::i18n::t("Value is taken from the calculator display."), 13.0 * d);
            }
            Screen::PickUnit { from } => {
                let c = self.s.conv_cat;
                let sel = if from { self.s.conv_from[c] } else { self.s.conv_to[c] };
                for (i, n) in convert::unit_names(c, &self.s.rates).iter().enumerate() {
                    row(out, &mut y, n, if i == sel { "✓" } else { "" }, Action::PickUnit(i));
                }
            }
            Screen::Settings => {
                let ln = if self.s.lang < 0 { crate::i18n::t("Phone language") } else { crate::i18n::LANG_NAMES[self.s.lang as usize] };
                row(out, &mut y, crate::i18n::t("Language"), ln, Action::CycleLang);
                row(out, &mut y, crate::i18n::t("Theme"), [crate::i18n::t("Automatic"), crate::i18n::t("Light"), crate::i18n::t("Dark")][self.s.theme as usize % 3], Action::CycleTheme);
                row(out, &mut y, crate::i18n::t("Vibrate on tap"), if self.s.vibrate { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::ToggleVibrate);
                row(out, &mut y, crate::i18n::t("Angle unit"), if self.s.degrees { crate::i18n::t("Degrees") } else { crate::i18n::t("Radians") }, Action::ToggleAngle);
                row(out, &mut y, crate::i18n::t("Scientific mode"), if self.s.sci { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::ToggleSci);
                row(out, &mut y, crate::i18n::t("Decimal sign"), if self.s.decimal == 1 { "." } else { "," }, Action::Set("decimal"));
                row(out, &mut y, crate::i18n::t("Digit grouping"), [crate::i18n::t("Space"), crate::i18n::t("None"), if self.s.decimal == 1 { "," } else { "." }, "'"][self.s.grouping as usize % 4], Action::Set("grouping"));
                let fixed = if self.s.fixed < 0 { crate::i18n::t("Automatic").to_string() } else { self.s.fixed.to_string() };
                row(out, &mut y, crate::i18n::t("Decimal places"), &fixed, Action::Set("fixed"));
                row(out, &mut y, crate::i18n::t("Result in words"), if self.s.words { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("words"));
                heading(out, &mut y, crate::i18n::t("Look"));
                let acc = [crate::i18n::t("Classic"), crate::i18n::t("Blue"), crate::i18n::t("Green"), crate::i18n::t("Orange"), crate::i18n::t("Purple"), crate::i18n::t("Red"), crate::i18n::t("Phone colors")][self.s.accent as usize % 7];
                row(out, &mut y, crate::i18n::t("Color"), acc, Action::Set("accent"));
                row(out, &mut y, crate::i18n::t("High contrast"), if self.s.contrast { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("contrast"));
                row(out, &mut y, crate::i18n::t("Button size"), [crate::i18n::t("Normal"), crate::i18n::t("Large"), crate::i18n::t("Small")][self.s.key_size as usize % 3], Action::Set("keys"));
                row(out, &mut y, crate::i18n::t("Text size"), [crate::i18n::t("Normal"), crate::i18n::t("Large"), crate::i18n::t("Extra large"), crate::i18n::t("Small")][self.s.font_size as usize % 4], Action::Set("font"));
                row(out, &mut y, crate::i18n::t("One-hand mode"), [crate::i18n::t("Off"), crate::i18n::t("Right hand"), crate::i18n::t("Left hand")][self.s.one_hand as usize % 3], Action::Set("onehand"));
                row(out, &mut y, crate::i18n::t("Press animation"), if self.s.animations { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("anim"));
                row(out, &mut y, crate::i18n::t("App icon"), [crate::i18n::t("Classic"), crate::i18n::t("Blue"), crate::i18n::t("Dark")][self.s.icon as usize % 3], Action::Set("icon"));
                heading(out, &mut y, crate::i18n::t("Behavior"));
                row(out, &mut y, crate::i18n::t("Sound on tap"), if self.s.sound { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("sound"));
                row(out, &mut y, crate::i18n::t("Vibration strength"), &format!("{} ms", self.s.vib_ms), Action::Set("vib"));
                row(out, &mut y, crate::i18n::t("Read results aloud"), if self.s.speak { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("speak"));
                row(out, &mut y, crate::i18n::t("Start screen"), [crate::i18n::t("Calculator"), crate::i18n::t("Converter"), crate::i18n::t("Tools"), crate::i18n::t("History")][self.s.start_screen as usize % 4], Action::Set("start"));
                row(out, &mut y, crate::i18n::t("Floating calculator"), "›", Action::Set("floating"));
                heading(out, &mut y, crate::i18n::t("Updates"));
                row(out, &mut y, crate::i18n::t("Check for updates in the background"), if self.s.bg_check { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("bgcheck"));
                row(out, &mut y, crate::i18n::t("Beta versions"), if self.s.beta { crate::i18n::t("On") } else { crate::i18n::t("Off") }, Action::Set("beta"));
                row(out, &mut y, crate::i18n::t("Check for updates"), crate::i18n::t("Check"), Action::CheckUpdate);
                row(out, &mut y, crate::i18n::t("What's new"), "›", Action::Set("whatsnew"));
                row(out, &mut y, crate::i18n::t("Report a problem"), "›", Action::Set("bug"));
                row(out, &mut y, crate::i18n::t("Version"), version(), Action::None);
                para(out, &mut y, &crate::i18n::tf("Updates: github.com/{}\nCalculator written in Rust.", &[&crate::update::UPDATE_REPO]), 13.0 * d);
            }
            Screen::Tools => {
                row(out, &mut y, crate::i18n::t("Word problem"), "›", Action::Nav(Screen::Ask));
                row(out, &mut y, crate::i18n::t("VAT and tips"), "›", Action::Nav(Screen::Vat));
                row(out, &mut y, crate::i18n::t("Graph"), "›", Action::Nav(Screen::Graph));
                for (i, td) in crate::tools::TOOLS.iter().enumerate() { row(out, &mut y, crate::i18n::t(td.name), "›", Action::OpenTool(i)); }
            }
            Screen::Tool => {
                let tool = self.tool;
                let def = &crate::tools::TOOLS[tool];
                if !def.modes.is_empty() {
                    let items: Vec<(String, Action, bool)> = def.modes.iter().enumerate().map(|(i, m)| (crate::i18n::t(m).to_string(), Action::ToolMode(i), i == self.tool_mode)).collect();
                    chips(out, &mut y, &items);
                }
                let vis = crate::tools::visible_fields(tool, self.tool_mode);
                let grid = tool == crate::tools::T_SYSTEMS || tool == crate::tools::T_MATRIX;
                let cols = if tool == crate::tools::T_SYSTEMS { if self.tool_mode == 0 { 3 } else { 4 } } else if tool == crate::tools::T_MATRIX { if self.tool_mode == 0 { 2 } else { 3 } } else { 1 };
                if tool == crate::tools::T_SYSTEMS { para(out, &mut y, if self.tool_mode == 0 { "a1·x + b1·y = c1\na2·x + b2·y = c2" } else { "a·x + b·y + c·z = d" }, 14.0 * d); }
                let cw = w / cols as f32;
                for (k, f) in vis.iter().enumerate() {
                    let (cx, cy) = if grid { (x + (k % cols) as f32 * cw, y + (k / cols) as f32 * 64.0 * d) } else { (x, y + k as f32 * 64.0 * d) };
                    let r = Rect::new(cx + if grid { 3.0 * d } else { 0.0 }, cy, if grid { cw - 6.0 * d } else { w }, 58.0 * d);
                    let focused = *f == self.focus;
                    out.push(W::Button { r, label: String::new(), px: 0.0, kind: if focused { Kind::ChipOn } else { Kind::Row }, action: Action::Focus(*f) });
                    let (lc, vc) = if focused { (t.chip_on_text, t.chip_on_text) } else { (t.dim, t.text) };
                    let label = crate::i18n::t(crate::tools::field_label(tool, self.tool_mode, *f));
                    let raw = &self.fields[*f];
                    let shown = if tool == crate::tools::T_BASES { raw.clone() } else if raw.is_empty() { String::new() } else { expr::display_raw(&raw.replacen('-', "−", 1).replace('−', "-")) };
                    if grid {
                        out.push(W::Text { r: Rect::new(r.x + 8.0 * d, r.y + 2.0 * d, r.w - 16.0 * d, 20.0 * d), text: label.into(), px: 12.0 * d, bold: false, c: lc, align: 0, wrap: false });
                        out.push(W::Text { r: Rect::new(r.x + 8.0 * d, r.y + 22.0 * d, r.w - 16.0 * d, 32.0 * d), text: shown, px: 17.0 * d, bold: true, c: vc, align: 2, wrap: false });
                    } else {
                        out.push(W::Text { r: Rect::new(r.x + 12.0 * d, r.y, r.w * 0.6, r.h), text: label.into(), px: 15.0 * d, bold: false, c: lc, align: 0, wrap: false });
                        out.push(W::Text { r: Rect::new(r.x + r.w * 0.4, r.y, r.w * 0.6 - 12.0 * d, r.h), text: if focused { format!("{shown}|") } else { shown }, px: 18.0 * d, bold: true, c: vc, align: 2, wrap: false });
                    }
                }
                let n_rows = if grid { (vis.len() + cols - 1) / cols } else { vis.len() };
                y += n_rows as f32 * 64.0 * d + 6.0 * d;
                if def.list && !self.list.is_empty() {
                    para(out, &mut y, &self.list.iter().map(|v| display_num(*v)).collect::<Vec<_>>().join(";  "), 14.0 * d);
                }
                let outs = if tool == crate::tools::T_BASES { crate::tools::bases(&self.fields[0], self.tool_mode) } else { crate::tools::compute(tool, self.tool_mode, &self.field_values(), &self.list, self.seed) };
                heading(out, &mut y, crate::i18n::t("Result"));
                for (label, val) in outs {
                    if label.is_empty() { para(out, &mut y, &val, 13.0 * d); continue; }
                    if val.contains('\n') {
                        heading(out, &mut y, &label);
                        for line in val.lines() { out.push(W::Text { r: Rect::new(x, y, w, 30.0 * d), text: line.into(), px: 18.0 * d, bold: true, c: t.text, align: 0, wrap: false }); y += 32.0 * d; }
                        continue;
                    }
                    let num = val.trim_end_matches(" €").trim_end_matches(" %").trim_end_matches(" l").replace(GROUPSP, "").replace('−', "-").replace(expr::dec_char(), ".");
                    let act = match num.parse::<f64>() { Ok(v) => Action::UseValue(v), Err(_) => Action::None };
                    row(out, &mut y, &label, &val, act);
                }
                para(out, &mut y, crate::i18n::t("Tap a result to use it in the calculator."), 12.0 * d);
            }
            Screen::Ask => {
                let typing = self.text_target.map(|t| t.0 == 2).unwrap_or(false);
                para(out, &mut y, crate::i18n::t("Say or type a problem in words, e.g. \"15 percent of 240\" or \"a book costs 12 euros, how much do 3 cost\"."), 14.0 * d);
                let tr = Rect::new(x, y, w, 96.0 * d);
                out.push(W::Button { r: tr, label: String::new(), px: 0.0, kind: if typing { Kind::ChipOn } else { Kind::Row }, action: if typing { Action::EndText } else { Action::StartText(2, 0) } });
                let tt = if self.ask_text.is_empty() && !typing { format!("{}…", crate::i18n::t("Tap to type")) } else { format!("{}{}", self.ask_text, if typing { "|" } else { "" }) };
                out.push(W::Text { r: tr.inset(12.0 * d, 8.0 * d), text: tt, px: 16.0 * d, bold: false, c: if typing { t.chip_on_text } else { t.text }, align: 0, wrap: true });
                y += 104.0 * d;
                chips(out, &mut y, &[(crate::i18n::t("Speak").to_string(), Action::Set("ask_voice"), false), (crate::i18n::t("Paste").to_string(), Action::Set("ask_paste"), false), (crate::i18n::t("Clear").to_string(), Action::Set("ask_clear"), false)]);
                let sr = Rect::new(x, y, w, 52.0 * d);
                out.push(W::Button { r: sr, label: crate::i18n::t("Solve").into(), px: 17.0 * d, kind: Kind::Eq, action: Action::Set("ask_solve") });
                y += 62.0 * d;
                if !self.ask_status.is_empty() { para(out, &mut y, &self.ask_status.clone(), 14.0 * d); }
                if let Some((e, a, nano)) = self.ask_out.clone() {
                    heading(out, &mut y, crate::i18n::t("Result"));
                    let rr = Rect::new(x, y, w, 40.0 * d);
                    out.push(W::Text { r: rr, text: format!("{e} ="), px: 18.0 * d, bold: false, c: t.dim, align: 0, wrap: true }); y += 42.0 * d;
                    out.push(W::Text { r: Rect::new(x, y, w, 48.0 * d), text: a, px: 30.0 * d, bold: true, c: t.text, align: 0, wrap: false }); y += 54.0 * d;
                    para(out, &mut y, if nano { crate::i18n::t("Understood by Gemini Nano on this phone, calculated by the app. Check that the expression matches the problem.") } else { crate::i18n::t("Understood offline by the app. Check that the expression matches the problem.") }, 13.0 * d);
                    row(out, &mut y, crate::i18n::t("Open in calculator"), "›", Action::Set("ask_open"));
                }
                heading(out, &mut y, "Gemini Nano");
                let ns = match self.nano.as_str() {
                    "available" => crate::i18n::t("Ready on this phone. Problems are understood on the phone, nothing is sent to the internet."),
                    "downloadable" | "downloading" => crate::i18n::t("This phone supports it, but the model is still downloading. Until then the offline reader is used."),
                    "" => crate::i18n::t("Checking…"),
                    _ => crate::i18n::t("Not supported on this phone. The app uses its own offline reader instead (simple problems only)."),
                };
                para(out, &mut y, ns, 13.0 * d);
            }
            Screen::Graph => {
                let toks = self.graph_tokens();
                if toks.is_empty() {
                    para(out, &mut y, crate::i18n::t("Type an expression with x in the calculator (e.g. x² − 2x − 3), then open Graph."), 15.0 * d);
                } else {
                    let ev = self.ev();
                    para(out, &mut y, &format!("y = {}", display_tokens(&toks)), 16.0 * d);
                    let gh = w * 0.9;
                    let r = Rect::new(x, y, w, gh);
                    out.push(W::Fill { r, c: t.chip, radius: 8.0 * d });
                    let span = self.graph_span;
                    let (x0, x1) = (-span, span);
                    let n = 400;
                    let mut ys = vec![];
                    for i in 0..=n { let xv = x0 + (x1 - x0) * i as f64 / n as f64; ys.push((xv, ev.f_at(&toks, xv))); }
                    let fin: Vec<f64> = ys.iter().filter_map(|p| p.1).filter(|v| v.is_finite()).collect();
                    let (mut y0, mut y1) = (-span, span);
                    if !fin.is_empty() {
                        let mut sorted = fin.clone(); sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let lo = sorted[sorted.len() / 20]; let hi = sorted[sorted.len() - 1 - sorted.len() / 20];
                        if hi > lo { let m = (hi - lo) * 0.15; y0 = lo - m; y1 = hi + m; }
                        if y0 > 0.0 { y0 = -m1(y1); } if y1 < 0.0 { y1 = m1(y0); }
                    }
                    fn m1(v: f64) -> f64 { v.abs() * 0.1 }
                    let sx = |xv: f64| r.x + ((xv - x0) / (x1 - x0)) as f32 * r.w;
                    let sy = |yv: f64| r.y + r.h - ((yv - y0) / (y1 - y0)) as f32 * r.h;
                    let mut lines: Vec<(u8, Vec<(f32, f32)>)> = vec![(2, vec![(r.x, sy(0.0)), (r.right(), sy(0.0))]), (2, vec![(sx(0.0), r.y), (sx(0.0), r.bottom())])];
                    let mut cur: Vec<(f32, f32)> = vec![];
                    for (xv, yv) in &ys {
                        match yv { Some(v) if v.is_finite() && *v > y0 - (y1 - y0) && *v < y1 + (y1 - y0) => cur.push((sx(*xv), sy(*v))), _ => { if cur.len() > 1 { lines.push((1, std::mem::take(&mut cur))); } else { cur.clear(); } } }
                    }
                    if cur.len() > 1 { lines.push((1, cur)); }
                    out.push(W::Plot { r, lines });
                    out.push(W::Text { r: Rect::new(r.x + 6.0 * d, r.y + 4.0 * d, r.w, 18.0 * d), text: format!("y: {} … {}", display_num(y1), display_num(y0)), px: 11.0 * d, bold: false, c: t.dim, align: 0, wrap: false });
                    out.push(W::Text { r: Rect::new(r.x, r.bottom() - 20.0 * d, r.w - 6.0 * d, 18.0 * d), text: format!("x: {} … {}", display_num(x0), display_num(x1)), px: 11.0 * d, bold: false, c: t.dim, align: 2, wrap: false });
                    y += gh + 8.0 * d;
                    chips(out, &mut y, &[("−".into(), Action::Set("gzoom-"), false), ("+".into(), Action::Set("gzoom+"), false)]);
                    let f = |xv: f64| ev.f_at(&toks, xv);
                    let roots = expr::numeric_roots(&f, x0, x1);
                    if !roots.is_empty() { row(out, &mut y, crate::i18n::t("Zeros"), &roots.iter().take(4).map(|v| display_num(*v)).collect::<Vec<_>>().join("; "), Action::None); }
                    if let Some(v0) = f(0.0) { row(out, &mut y, "y(0)", &display_num(v0), Action::UseValue(v0)); }
                    if let Ok(pl) = ev.eval_poly(&toks) {
                        row(out, &mut y, "f′(x)", &poly_text(&pl.deriv()), Action::None);
                        let mut ic = [0.0; 5]; for i in 0..4 { ic[i + 1] = pl.0[i] / (i + 1) as f64; }
                        row(out, &mut y, "∫ f(x) dx", &format!("{} + C", poly_text5(&ic)), Action::None);
                    } else {
                        let dv = |xv: f64| Some((f(xv + 1e-5)? - f(xv - 1e-5)?) / 2e-5);
                        if let Some(v) = dv(1.0) { row(out, &mut y, "f′(1)", &display_num((v * 1e6).round() / 1e6), Action::None); }
                    }
                    if let Some(iv) = expr::integrate(&f, 0.0, 1.0) { row(out, &mut y, "∫₀¹ f(x) dx", &display_num((iv * 1e9).round() / 1e9), Action::UseValue(iv)); }
                }
            }
            Screen::Photo => {
                let bw = (w - 10.0 * d) / 2.0;
                out.push(W::Button { r: Rect::new(x, y, bw, 48.0 * d), label: crate::i18n::t("Take photo").into(), px: 16.0 * d, kind: Kind::ChipOn, action: Action::TakePhoto });
                out.push(W::Button { r: Rect::new(x + bw + 10.0 * d, y, bw, 48.0 * d), label: crate::i18n::t("From gallery").into(), px: 16.0 * d, kind: Kind::Chip, action: Action::OpenGallery });
                y += 60.0 * d;
                match &self.photo {
                    PhotoState::Idle => para(out, &mut y, crate::i18n::t("Take a photo of a printed problem, e.g. \"2x + 3 = 11\" or \"125 : 5 =\". The app finds the problem in the photo and solves it.\n\nWorks offline."), 15.0 * d),
                    PhotoState::Working => para(out, &mut y, crate::i18n::t("Recognizing… (a few seconds)"), 16.0 * d),
                    PhotoState::Failed(m) => para(out, &mut y, m, 15.0 * d),
                    PhotoState::Done { text, tokens, img, others, .. } => {
                        let ih = (w * img.height() as f32 / img.width().max(1) as f32).min(260.0 * d);
                        out.push(W::Photo { r: Rect::new(x, y, w, ih) });
                        y += ih + 10.0 * d;
                        row(out, &mut y, crate::i18n::t("Recognized"), &text.clone(), Action::None);
                        row(out, &mut y, crate::i18n::t("Problem"), &display_tokens(tokens), Action::UseTokens);
                        let (ans, steps) = self.photo_answer(tokens);
                        for s in &steps { para(out, &mut y, s, 14.0 * d); }
                        out.push(W::Text { r: Rect::new(x, y, w, 50.0 * d), text: ans, px: 26.0 * d, bold: true, c: t.text, align: 0, wrap: true });
                        y += 58.0 * d;
                        out.push(W::Button { r: Rect::new(x, y, w, 44.0 * d), label: crate::i18n::t("Edit in calculator").into(), px: 15.0 * d, kind: Kind::Chip, action: Action::UseTokens });
                        y += 52.0 * d;
                        let bw2 = (w - 10.0 * d) / 2.0;
                        out.push(W::Button { r: Rect::new(x, y, bw2, 44.0 * d), label: crate::i18n::t("Share").into(), px: 15.0 * d, kind: Kind::Chip, action: Action::Set("photo_share") });
                        out.push(W::Button { r: Rect::new(x + bw2 + 10.0 * d, y, bw2, 44.0 * d), label: crate::i18n::t("Solve as word problem").into(), px: 14.0 * d, kind: Kind::Chip, action: Action::Set("photo_words") });
                        y += 52.0 * d;
                        if !others.is_empty() {
                            heading(out, &mut y, crate::i18n::t("Other problems in the photo"));
                            for (i, (t2, _)) in others.iter().enumerate() { row(out, &mut y, &t2.clone(), "›", Action::PhotoPick(i)); }
                        }
                        para(out, &mut y, crate::i18n::t("If recognition is wrong, tap \"Edit in calculator\", fix it and press \"=\"."), 13.0 * d);
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
        W::Plot { r, lines } => { r.y += dy; for (_, l) in lines { for p in l { p.1 += dy; } } }
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
                let mut face = if matches!(kind, Kind::Digit | Kind::Op | Kind::Eq | Kind::Sci) { r.inset(2.0 * d, 3.0 * d) } else { *r };
                if is_pressed && app.s.animations { face = face.inset(face.w * 0.04, face.h * 0.05); }
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
            W::Plot { r, lines } => {
                let c = |k: u8| match k { 0 => t.text, 1 => [0x1E, 0x88, 0xE5], _ => t.line };
                let clip2 = r.intersect(&clip);
                for (k, pts) in lines {
                    let col = c(*k);
                    let th = if *k == 2 { 1.0 * d } else { 2.2 * d };
                    for w2 in pts.windows(2) {
                        let ((x0, y0), (x1, y1)) = (w2[0], w2[1]);
                        let n = ((x1 - x0).abs().max((y1 - y0).abs()) / (0.7 * d)).ceil().max(1.0) as i32;
                        for i in 0..=n {
                            let f = i as f32 / n as f32;
                            let (px, py) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                            let rr = Rect::new(px - th / 2.0, py - th / 2.0, th, th);
                            let ri = rr.intersect(&clip2);
                            if ri.w > 0.0 && ri.h > 0.0 { cv.round_rect(ri, 0.0, col, clip2); }
                        }
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
    // user / system text size, but never taller than the box
    let mut px = (px * font_scale()).min(r.h.max(px));
    let mut lines = vec![text.to_string()];
    if wrap {
        // shrink down to 60 % before wrapping (keeps long numbers on one line when possible)
        let w0 = fonts.width(text, px, bold);
        if w0 > r.w && text.chars().count() <= 40 { px = (px * r.w / w0).max(px * 0.6); }
        lines = fonts.wrap(text, px, bold, r.w);
    } else {
        let w0 = fonts.width(text, px, bold);
        if w0 > r.w && (align != 0 || font_scale() > 1.0) { px *= r.w / w0; }
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

const GROUPSP: [char; 4] = ['\u{202F}', ' ', '\'', '\u{A0}'];

fn poly_text5(c: &[f64]) -> String {
    let mut parts: Vec<String> = vec![];
    for i in (0..c.len()).rev() {
        let v = c[i];
        if v.abs() < 1e-12 { continue; }
        let sup = ["", "", "²", "³", "⁴"][i];
        let mag = v.abs();
        let coef = if (mag - 1.0).abs() < 1e-12 && i > 0 { String::new() } else { expr::to_fraction(mag).filter(|f| f.1 > 1 && f.1 <= 12).map(|f| format!("{}/{}·", f.0, f.1)).unwrap_or_else(|| display_num(mag)) };
        let term = match i { 0 => coef, 1 => format!("{coef}x"), _ => format!("{coef}x{sup}") };
        if parts.is_empty() { parts.push(if v < 0.0 { format!("−{term}") } else { term }); } else { parts.push(format!("{} {term}", if v < 0.0 { "−" } else { "+" })); }
    }
    if parts.is_empty() { "0".into() } else { parts.join(" ") }
}
fn poly_text(p: &expr::Poly) -> String { poly_text5(&p.0) }

pub const WHATS_NEW: &str = "• English plus 6 more languages (Settings → Language)\n• Scientific: 2nd page with inverse and hyperbolic functions, any root and log base, mod, nCr/nPr, EE, fractions, complex numbers (i)\n• Equations: cubic, any function, inequalities (<, >), systems\n• Graphs with derivative and integral\n• Tools: loan, deposit, salary (LT), dates, time, fuel, BMI, bill split, discounts, statistics, matrices, number systems, random/dice\n• Word problems (Gemini Nano when the phone supports it, otherwise offline)\n• History: search, star, notes, export, backup, swipe to delete\n• Look: colors, phone colors, high contrast, button and text size, one-hand mode, icon choice\n• Undo/redo, cursor, Ans, M1-M5, keyboard support\n• Home screen widget, quick tile, floating calculator, voice input, reading results aloud, TalkBack support, background update check";

impl App {
    /// Offline or model result for the word-problem screen.
    pub fn ask_result(&mut self, r: Option<(String, bool)>) {
        match r.and_then(|(e, nano)| expr::eval_text(&e).map(|v| (e, v, nano))) {
            Some((e, v, nano)) => { self.ask_status.clear(); self.ask_out = Some((e.replace('*', "×").replace('/', "÷"), expr::display_num(v), nano)); }
            None => { self.ask_out = None; self.ask_status = crate::i18n::t("Could not understand the problem. Try writing it with numbers, e.g. \"12 * 3 + 4\".").into(); }
        }
    }
}
