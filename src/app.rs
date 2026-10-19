//! Application state, key handling and screen layout (as a list of widgets).

use crate::convert::{self, Rates, CATEGORIES, CURRENCY_CAT};
use crate::draw::{Canvas, Fonts, Rect, Rgb};
use crate::expr::{self, display_num, display_tokens, plain, Evaluator, SolveError, Tok};
use crate::market::{MarketKind, Quote};
use serde::{Deserialize, Serialize};

pub const VERSION: &str = include_str!("../VERSION");
pub fn version() -> &'static str {
    VERSION.trim()
}

// ---------------- persistent data ----------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistItem {
    pub expr: String,
    pub result: String,
    pub value: Option<f64>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub photo: bool,
}
impl HistItem {
    pub fn new(expr: String, result: String, value: Option<f64>) -> Self {
        HistItem {
            expr,
            result,
            value,
            pinned: false,
            note: String::new(),
            photo: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub theme: u8, // 0 auto, 1 light, 2 dark
    pub vibrate: bool,
    pub degrees: bool,
    pub sci: bool,
    pub memory: f64, // (old single memory; migrated into mem[0])
    pub history: Vec<HistItem>,
    pub conv_cat: usize,
    pub conv_from: Vec<usize>,
    pub conv_to: Vec<usize>,
    pub rates: Option<Rates>,
    pub tip: usize,
    pub people: usize,
    pub ans: f64,
    pub accent: u8, // 0 classic, 1 blue, 2 green, 3 orange, 4 purple, 5 red, 6 phone colors (Material You)
    pub key_size: u8, // 0 normal, 1 large, 2 small
    pub font_size: u8, // 0 normal, 1 large, 2 extra large, 3 small
    pub animations: bool,
    pub sound: bool,
    pub one_hand: u8, // 0 off, 1 right hand, 2 left hand
    pub icon: u8,     // 0 classic, 1 blue, 2 dark
    pub contrast: bool,
    pub start_screen: u8, // 0 calculator, 1 converter, 2 tools, 3 history
    pub speak: bool,
    pub beta: bool,
    pub bg_check: bool,
    pub vib_ms: u8, // vibration length in ms
    pub last_version: String,
    pub fav_cats: Vec<usize>,
    pub mem: Vec<f64>, // M1-M5
    pub mem_slot: usize,
    pub decimal: u8,  // 0 comma, 1 dot
    pub grouping: u8, // 0 space, 1 none, 2 dot/comma, 3 apostrophe
    pub fixed: i32,   // -1 automatic, else digits after the decimal sign
    pub words: bool,
    pub formulas: Vec<crate::learning::Formula>,
    pub quiz_best: [usize;3],
    pub lang: i32, // -1 = phone language, otherwise index into i18n::LANGS (0 = English)
    pub game_best: [u64; 5], // 2048, fifteen (min moves), tetris, snake, minesweeper wins
    pub quiz_best_ext: [usize; 2], // physics, geometry
    pub quiz_streak_best: usize,
    pub custom_accent: u32, // 0xRRGGBB, used when accent == 7
}

impl Default for Saved {
    fn default() -> Self {
        Saved {
            theme: 0,
            vibrate: true,
            degrees: true,
            sci: false,
            memory: 0.0,
            history: vec![],
            conv_cat: 1,
            conv_from: vec![0, 3, 2, 0, 1, 1, 1],
            conv_to: vec![1, 7, 5, 1, 3, 3, 2],
            rates: None,
            tip: 1,
            people: 1,
            ans: 0.0,
            accent: 0,
            key_size: 0,
            font_size: 0,
            animations: true,
            sound: false,
            one_hand: 0,
            icon: 0,
            contrast: false,
            start_screen: 0,
            speak: false,
            beta: false,
            bg_check: true,
            vib_ms: 20,
            last_version: String::new(),
            fav_cats: vec![],
            mem: vec![0.0; 5],
            mem_slot: 0,
            decimal: 0,
            grouping: 0,
            fixed: -1,
            words: false,
            formulas: vec![],
            quiz_best: [0;3],
            lang: 0,
            game_best: [0; 5],
            quiz_best_ext: [0; 2],
            quiz_streak_best: 0,
            custom_accent: 0x1E6FD9,
        }
    }
}

// ---------------- runtime state ----------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Calc,
    History,
    Vat,
    Converter,
    PickUnit { from: bool },
    Settings,
    Photo,
    Gallery,
    Tools,
    Tool,
    Markets,
    Graph,
    Ask,
    WhatsNew,
    MarketDetail,
    Periodic,
    Element,
    Constants,
    Steps,
    Graph3D,
    Quiz,
    Formulas,
    StatsCharts,
    Games,
    Game(usize),
    Prog,
    Cheat,
}

pub enum PhotoState {
    Idle,
    Working,
    Done {
        img: image::RgbImage,
        bbox: Option<[f32; 4]>,
        text: String,
        tokens: Vec<Tok>,
        others: Vec<(String, [f32; 4])>,
    },
    Failed(String),
}

pub struct GalleryItem {
    pub id: i64,
    pub thumb: Option<image::RgbImage>,
    pub label: String,
}

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
    MarketKind(MarketKind),
    MarketRefresh,
    MarketOpen(usize),
    MarketRange(usize),
    QuizMode(usize),
    QuizAnswer(usize),
    FormulaOpen(usize),
    FormulaDelete(usize),
    ShareHistoryImage(usize),
    MarketPoint(usize),
    EditCursor(usize),
    ElementOpen(usize),
    Surface(usize),
    Tick,
    GameNav(usize),
    GameNew,
    GameMove(u8),
    GameTap(usize),
    ProgKey(&'static str),
    None,
}

/// Side effects the platform layer performs.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    InputDialog(String,String,u8),
    ShareImage(String,String),
    Floating,
    BgCheck(bool),
    Listen,
    Solve(String, bool),
    NanoStatus,
    ReportBug,
    SetIcon(u8),
    Click,
    Keyboard(bool),
    Share(String),
    SaveFile(String, String),
    Speak(String),
    Vibrate,
    Copy(String),
    RequestPaste,
    Save,
    FetchRates,
    FetchRateHistory,
    FetchMarkets(MarketKind),
    FetchMarketHistory(MarketKind, String, usize),
    CheckUpdate,
    UpdateTap,
    TakePhoto,
    OpenGallery,
    LoadGallery(i64),
}

pub struct App {
    pub s: Saved,
    pub tokens: Vec<Tok>,
    pub just_evaluated: bool,
    pub result: Option<(String, String)>, // (expression line, result text) after "="
    pub screen: Screen,
    pub nav_stack: Vec<Screen>,
    pub edit_text: Option<String>,
    pub edit_cursor: usize,
    pub market_point: usize,
    pub quiz_mode: usize,
    pub quiz_streak: usize,
    pub quiz_question: Option<crate::learning::Question>,
    pub quiz_count: usize,
    pub quiz_score: usize,
    pub quiz_feedback: String,
    pub quiz_answered: bool,
    pub formula_sel: Option<usize>,
    pub formula_result: String,
    pub stats_data: Vec<f64>,
    pub stats_pie: bool,
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
    pub market_kind: MarketKind,
    pub market_quotes: Vec<Quote>,
    pub market_status: String,
    pub market_sel: Option<Quote>,
    pub market_range: usize,
    pub market_hist: Vec<(f64, f64)>,
    pub market_hist_status: String,
    /// System bar / cutout insets in pixels: left, top, right, bottom.
    pub insets: [f32; 4],
    pub elem_sel: usize,
    pub game: Option<crate::games::Game>,
    pub game_kind: usize,
    pub mines_flag: bool,
    pub prog: crate::prog::Prog,
    pub graph_toks: Vec<Vec<Tok>>,
    pub steps_src: Option<Vec<Tok>>,
    pub list2: Vec<f64>,
    pub g3_surface: usize,
    pub g3_yaw: f64,
    pub g3_pitch: f64,
    pub g3_span: f64,
}

impl Default for App {
    fn default() -> Self {
        Self::new(Saved::default())
    }
}

fn is_value_end(t: Option<&Tok>) -> bool {
    matches!(
        t,
        Some(
            Tok::Num(_)
                | Tok::RParen
                | Tok::Pi
                | Tok::E
                | Tok::X
                | Tok::I
                | Tok::Ans
                | Tok::Post(_)
        )
    )
}

impl App {
    pub fn new(s: Saved) -> Self {
        let mut s = s;
        if s.mem.len() < 5 {
            s.mem.resize(5, 0.0);
        }
        if s.memory != 0.0 && s.mem[0] == 0.0 {
            s.mem[0] = s.memory;
            s.memory = 0.0;
        }
        s.mem_slot = s.mem_slot.min(4);
        for (i, (f, t)) in convert::DEFAULTS.iter().enumerate().take(CATEGORIES.len()) {
            if s.conv_from.len() <= i {
                s.conv_from.push(*f);
            }
            if s.conv_to.len() <= i {
                s.conv_to.push(*t);
            }
        }
        s.conv_cat = s.conv_cat.min(CATEGORIES.len() - 1);
        let a = App {
            s,
            tokens: vec![],
            just_evaluated: false,
            result: None,
            screen: Screen::Calc,
            nav_stack: vec![],
            edit_text: None,
            edit_cursor: 0,
            market_point: 0,
            quiz_mode:0,quiz_streak:0,quiz_question:None,quiz_count:0,quiz_score:0,quiz_feedback:String::new(),quiz_answered:false,
            formula_sel:None,formula_result:String::new(),stats_data:vec![],stats_pie:false,
            scroll: 0.0,
            content_h: 0.0,
            popup: false,
            banner: None,
            toast: None,
            system_dark: false,
            system_lang: 0,
            photo: PhotoState::Idle,
            gallery: vec![],
            gallery_status: String::new(),
            rates_status: String::new(),
            cursor: None,
            undo: vec![],
            redo: vec![],
            show_frac: false,
            sci_page: 0,
            tool: 0,
            tool_mode: 0,
            fields: vec![],
            focus: 0,
            list: vec![],
            seed: 0x9E3779B97F4A7C15,
            graph_span: 10.0,
            market_kind: MarketKind::Crypto,
            market_quotes: vec![],
            market_status: String::new(),
            market_sel: None,
            market_range: 2,
            market_hist: vec![],
            market_hist_status: String::new(),
            insets: [0.0; 4],
            elem_sel: 0,
            game: None,
            game_kind: 0,
            mines_flag: false,
            prog: crate::prog::Prog::default(),
            graph_toks: vec![],
            steps_src: None,
            list2: vec![],
            g3_surface: 1,
            g3_yaw: 0.6,
            g3_pitch: 0.7,
            g3_span: 3.0,
            rate_hist: None,
            text_target: None,
            search: String::new(),
            ask_text: String::new(),
            ask_out: None,
            ask_status: String::new(),
            nano: String::new(),
            restore_pending: false,
            system_accent: None,
            system_font: 1.0,
            whats_new: false,
            crash: None,
        };
        a.apply_lang();
        a.apply_format();
        let mut a = a;
        a.screen = match a.s.start_screen {
            1 => Screen::Converter,
            2 => Screen::Tools,
            3 => Screen::History,
            _ => Screen::Calc,
        };
        // "What's new" after an update (not on the very first start)
        if a.s.last_version != version() {
            a.whats_new = !a.s.last_version.is_empty();
            if a.whats_new {
                a.screen = Screen::WhatsNew;
            }
            a.s.last_version = version().to_string();
        }
        a.apply_look();
        a
    }

    pub fn apply_look(&self) {
        crate::update::BETA.store(self.s.beta, std::sync::atomic::Ordering::Relaxed);
        let acc: Option<Rgb> = match self.s.accent {
            1 => Some([0x1E, 0x6F, 0xD9]),
            2 => Some([0x2E, 0x8B, 0x57]),
            3 => Some([0xF2, 0x8C, 0x28]),
            4 => Some([0x7E, 0x57, 0xC2]),
            5 => Some([0xD3, 0x2F, 0x2F]),
            6 => self.system_accent.or(Some([0x1E, 0x6F, 0xD9])),
            7 => {
                let c = self.s.custom_accent;
                Some([(c >> 16) as u8, (c >> 8) as u8, c as u8])
            }
            _ => None,
        };
        AMOLED.store(self.s.theme == 3, std::sync::atomic::Ordering::Relaxed);
        LOOK.store(
            acc.map(|c| 0x0100_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
                .unwrap_or(0),
            std::sync::atomic::Ordering::Relaxed,
        );
        CONTRAST.store(self.s.contrast, std::sync::atomic::Ordering::Relaxed);
        let user = [1.0, 1.15, 1.3, 0.9][self.s.font_size as usize % 4];
        FONT_SCALE.store(
            ((user * self.system_font.clamp(0.85, 1.3)) * 1000.0) as u32,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
    pub fn apply_format(&self) {
        expr::set_format(self.s.decimal, self.s.grouping, self.s.fixed);
    }
    pub fn apply_lang(&self) {
        crate::i18n::set_lang(if self.s.lang < 0 {
            self.system_lang
        } else {
            self.s.lang as usize
        });
    }
    pub fn dark(&self) -> bool {
        match self.s.theme {
            1 => false,
            2 | 3 => true,
            _ => self.system_dark,
        }
    }
    fn ev(&self) -> Evaluator {
        Evaluator {
            degrees: self.s.degrees,
            ans: self.s.ans,
            xval: None,
        }
    }

    /// The value the calculator currently shows (result, or live value of the expression).
    pub fn current_value(&self) -> Option<f64> {
        if self
            .tokens
            .iter()
            .any(|t| matches!(t, Tok::X | Tok::Equals | Tok::Cmp(_)))
        {
            return None;
        }
        self.ev().eval(&self.tokens).ok()
    }

    pub fn paste(&mut self, text: &str) {
        self.edit_text = None;
        if self.screen == Screen::Ask {
            self.ask_text = text.trim().chars().take(500).collect();
            self.ask_out = None;
            return;
        }
        if self.restore_pending {
            self.restore_pending = false;
            match serde_json::from_str::<Saved>(text.trim()) {
                Ok(mut s) if !s.history.is_empty() || s.lang != 0 || s.theme != 0 => {
                    s.rates = self.s.rates.take();
                    let sys = (self.system_dark, self.system_lang);
                    *self = App::new(s);
                    self.system_dark = sys.0;
                    self.system_lang = sys.1;
                    self.apply_lang();
                    self.screen = Screen::History;
                    self.toast = Some(crate::i18n::t("Backup restored").into());
                }
                _ => {
                    self.toast = Some(
                        crate::i18n::t(
                            "No backup in clipboard. Tap Backup first, then copy it back.",
                        )
                        .into(),
                    )
                }
            }
            return;
        }
        if let Some(raw) = expr::parse_pasted(text) {
            if self.just_evaluated || self.tokens.is_empty() {
                self.tokens = vec![Tok::Num(raw)];
                self.just_evaluated = false;
                self.result = None;
            } else if !is_value_end(self.tokens.last()) {
                self.tokens.push(Tok::Num(raw));
            } else {
                self.tokens.push(Tok::Op('×'));
                self.tokens.push(Tok::Num(raw));
            }
            self.toast = Some(crate::i18n::t("Pasted").into());
        } else {
            self.toast = Some(crate::i18n::t("No number in clipboard").into());
        }
    }

    pub fn quiz_best(&self, mode: usize) -> usize {
        if mode < 3 { self.s.quiz_best[mode] } else { self.s.quiz_best_ext[(mode - 3).min(1)] }
    }
    fn quiz_record(&mut self, mode: usize, score: usize) {
        if mode < 3 { self.s.quiz_best[mode] = self.s.quiz_best[mode].max(score); }
        else { let i = (mode - 3).min(1); self.s.quiz_best_ext[i] = self.s.quiz_best_ext[i].max(score); }
    }
    pub fn next_question(&mut self) {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut q=crate::learning::question(self.seed,self.quiz_mode);
        let shift=(self.seed%4) as usize;q.options.rotate_right(shift);q.answer=shift;
        self.quiz_question=Some(q);self.quiz_feedback.clear();self.quiz_answered=false;
    }
    pub fn dialog_result(&mut self,kind:u8,text:&str) {
        match kind {
            3 => {
                let mut lines=text.lines();let name=lines.next().unwrap_or("").trim();let expression=lines.collect::<Vec<_>>().join(" ");
                let f=crate::learning::Formula{name:name.into(),expression};
                let vars=f.variables();let values=vars.iter().map(|v|(v.clone(),1.)).collect::<Vec<_>>();
                if name.is_empty() || f.expression.is_empty() || name.len()>80 || f.expression.len()>500 || self.s.formulas.len()>=50 || f.evaluate(&values).is_err(){self.toast=Some("Check the name and formula (max 50 saved)".into());}
                else {self.s.formulas.push(f);}
            }
            4 => {
                if let Some(f)=self.formula_sel.and_then(|i|self.s.formulas.get(i)) {
                    let values:Result<Vec<_>,_>=text.lines().filter(|l|!l.trim().is_empty()).map(|line|{let (k,v)=line.split_once('=').ok_or("Use variable=value on each line")?;Ok::<_,&str>((k.trim().to_string(),v.trim().parse::<f64>().map_err(|_|"Check numeric values")?))}).collect();
                    self.formula_result=values.map_err(str::to_string).and_then(|v|f.evaluate(&v)).map(display_num).unwrap_or_else(|e|e);
                }
            }
            5 => match crate::learning::dataset(text){Ok(v)=>self.stats_data=v,Err(e)=>self.toast=Some(e)},
            6 => {
                self.graph_toks = text
                    .lines()
                    .filter_map(|l| expr::tokenize_edit(l.trim()))
                    .take(3)
                    .collect();
                if self.graph_toks.is_empty() {
                    self.toast = Some(crate::i18n::t("No function of x found. Example: x^2 - 2x - 3").into());
                }
            }
            7 => {
                let v = u32::from_str_radix(text.trim().trim_start_matches('#'), 16).unwrap_or(0);
                if text.trim().trim_start_matches('#').chars().count() == 6 && v <= 0xFFFFFF {
                    self.s.custom_accent = v;
                    self.s.accent = 7;
                    self.apply_look();
                } else {
                    self.toast = Some(crate::i18n::t("Use six hex digits, for example #1E6FD9").into());
                }
            }
            _=>{}
        }
    }

    pub fn navigate(&mut self, screen: Screen) {
        if self.screen != screen { self.nav_stack.push(self.screen); self.screen = screen; }
    }

    fn next_seed(&mut self) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.seed
    }
    pub fn new_game(&mut self, kind: usize) {
        let kind = kind.min(crate::games::GAME_COUNT - 1);
        let seed = self.next_seed();
        self.game = Some(crate::games::new_game(kind, seed));
        self.game_kind = kind;
        self.mines_flag = false;
    }
    fn save_best(&mut self, i: usize, v: u64, minimize: bool, fx: &mut Vec<Effect>) {
        let cur = self.s.game_best[i];
        let better = if minimize { v > 0 && (cur == 0 || v < cur) } else { v > cur };
        if better {
            self.s.game_best[i] = v;
            fx.push(Effect::Save);
        }
    }
    pub fn game_move(&mut self, c: u8, fx: &mut Vec<Effect>) {
        let mut moved2048 = None;
        let mut tetris_over = None;
        let Some(g) = &mut self.game else { return };
        match g {
            crate::games::Game::G2048(g) => {
                if g.mv(c as usize % 4) {
                    moved2048 = Some((g.score, g.over));
                }
            }
            crate::games::Game::Tetris(t) => match c {
                0 => {
                    t.mv(-1);
                }
                1 => {
                    t.mv(1);
                }
                2 => {
                    t.rotate();
                }
                3 => {
                    t.tick();
                }
                _ => {
                    t.hard_drop();
                    tetris_over = t.over.then_some(t.score);
                }
            },
            crate::games::Game::Snake(s) => s.turn(c as i32),
            _ => {}
        }
        if let Some((sc, over)) = moved2048 {
            self.save_best(0, sc, false, fx);
            if over {
                self.toast = Some(crate::i18n::t("Game over").into());
            }
        }
        if let Some(sc) = tetris_over {
            self.save_best(2, sc, false, fx);
        }
    }
    pub fn game_tap(&mut self, i: usize, fx: &mut Vec<Effect>) {
        let mut solved_moves = None;
        let mut mines_won = false;
        match &mut self.game {
            Some(crate::games::Game::Fifteen(f)) => {
                if f.tap(i) && f.solved() {
                    solved_moves = Some(f.moves);
                }
            }
            Some(crate::games::Game::Mines(m)) => {
                let (x, y) = (i % crate::games::MW, i / crate::games::MW);
                if self.mines_flag {
                    m.toggle_flag(x, y);
                } else {
                    m.reveal(x, y);
                    mines_won = m.won;
                }
            }
            _ => {}
        }
        if let Some(m) = solved_moves {
            self.save_best(1, m as u64, true, fx);
            self.toast = Some(crate::i18n::tf("Solved in {} moves!", &[&m]));
        }
        if mines_won {
            self.s.game_best[4] += 1;
            fx.push(Effect::Save);
        }
    }
    pub fn game_tick(&mut self, fx: &mut Vec<Effect>) {
        let mut best: Option<(usize, u64)> = None;
        match &mut self.game {
            Some(crate::games::Game::Tetris(t)) => {
                t.tick();
                if t.over {
                    best = Some((2, t.score));
                }
            }
            Some(crate::games::Game::Snake(s)) => {
                s.tick();
                if s.over {
                    best = Some((3, s.score as u64));
                }
            }
            _ => {}
        }
        if let Some((i, v)) = best {
            self.save_best(i, v, false, fx);
            self.toast = Some(crate::i18n::t("Game over").into());
        }
    }
    /// Gravity interval for the visible timed game, if any.
    pub fn tick_ms(&self) -> Option<u64> {
        if self.screen != Screen::Game(self.game_kind) {
            return None;
        }
        self.game.as_ref().and_then(crate::games::tick_ms)
    }

    pub fn cycle_setting(&mut self, k: &str) {
        let s = &mut self.s;
        match k {
            "decimal" => s.decimal = (s.decimal + 1) % 2,
            "grouping" => s.grouping = (s.grouping + 1) % 4,
            "fixed" => {
                s.fixed = match s.fixed {
                    -1 => 0,
                    0 => 2,
                    2 => 4,
                    4 => 6,
                    6 => 10,
                    _ => -1,
                }
            }
            "words" => s.words = !s.words,
            "accent" => s.accent = (s.accent + 1) % 8,
            "contrast" => s.contrast = !s.contrast,
            "keys" => s.key_size = (s.key_size + 1) % 3,
            "font" => s.font_size = (s.font_size + 1) % 4,
            "onehand" => s.one_hand = (s.one_hand + 1) % 3,
            "anim" => s.animations = !s.animations,
            "icon" => s.icon = (s.icon + 1) % 3,
            "sound" => s.sound = !s.sound,
            "vib" => {
                s.vib_ms = match s.vib_ms {
                    0..=10 => 20,
                    11..=20 => 40,
                    21..=40 => 70,
                    _ => 10,
                }
            }
            "speak" => s.speak = !s.speak,
            "start" => s.start_screen = (s.start_screen + 1) % 4,
            "beta" => s.beta = !s.beta,
            _ => {}
        }
        self.apply_format();
        self.apply_look();
    }

    /// Numeric values of the tool fields (NaN when empty or invalid).
    pub fn market_quote_count(&self) -> usize {
        self.market_quotes.len()
    }

    pub fn field_values(&self) -> Vec<f64> {
        self.fields
            .iter()
            .map(|f| f.replace(',', ".").parse::<f64>().unwrap_or(f64::NAN))
            .collect()
    }

    fn form_key(&mut self, k: &str) {
        let vis = crate::tools::visible_fields(self.tool, self.tool_mode);
        let hex = self.tool == crate::tools::T_BASES;
        let Some(f) = self.fields.get_mut(self.focus) else {
            return;
        };
        match k {
            "⌫" => {
                f.pop();
            }
            "C" => {
                f.clear();
            }
            "," => {
                if !hex && !f.contains('.') {
                    if f.is_empty() || f == "-" {
                        f.push('0');
                    }
                    f.push('.');
                }
            }
            "±" => {
                if f.starts_with('-') {
                    f.remove(0);
                } else {
                    f.insert(0, '-');
                }
            }
            "next" => {
                if !vis.is_empty() {
                    let p = vis.iter().position(|v| *v == self.focus).unwrap_or(0);
                    self.focus = vis[(p + 1) % vis.len()];
                }
            }
            "add" => {
                if self.tool == crate::tools::T_GRADES {
                    let g = self.fields[0].replace(',', ".").parse::<f64>();
                    let w = if self.fields[1].trim().is_empty() { Ok(1.0) } else { self.fields[1].replace(',', ".").parse::<f64>() };
                    if let (Ok(g), Ok(w)) = (g, w) {
                        if self.list.len() < 198 {
                            self.list.push(g);
                            self.list.push(w);
                        }
                        self.fields[0].clear();
                        self.fields[1].clear();
                        self.focus = 0;
                    }
                } else {
                    let v = f.replace(',', ".").parse::<f64>();
                    if let Ok(v) = v {
                        if self.list.len() < 200 {
                            self.list.push(v);
                        }
                        f.clear();
                    }
                }
            }
            "undo" => {
                self.list.pop();
                if self.tool == crate::tools::T_GRADES {
                    self.list.pop();
                }
            }
            "roll" => self.reroll(),
            "C_" => {
                f.push('C');
            }
            d => {
                if f.len() < 18 {
                    f.push_str(d);
                }
            }
        }
    }

    /// Processes a decoded photo (runs OCR; call from a worker thread with a clone of the image).
    pub fn photo_result(img: image::RgbImage) -> PhotoState {
        match crate::ocr::recognize_lines(&img) {
            Ok(lines) => {
                // a "-" at the very start of a line that touches the photo edge is usually the paper edge or a shadow
                let edge = img.width() as f32 * 0.03;
                let texts: Vec<String> = lines
                    .iter()
                    .map(|l| {
                        let t = l.text.trim_start();
                        if l.bbox[0] <= edge
                            && t.starts_with('-')
                            && t[1..]
                                .trim_start()
                                .starts_with(|c: char| c.is_ascii_digit())
                        {
                            t[1..].trim_start().to_string()
                        } else {
                            l.text.clone()
                        }
                    })
                    .collect();
                // every other line that also looks like a problem can be picked instead
                let mut others: Vec<(String, [f32; 4])> = vec![];
                match crate::ocr::best_index(&texts) {
                    Some(i) => {
                        let tokens = crate::ocr::to_tokens(&texts[i]);
                        if tokens.is_empty() {
                            return PhotoState::Failed(
                                crate::i18n::t("No problem found in the photo.").into(),
                            );
                        }
                        for (j, t) in texts.iter().enumerate() {
                            if j != i
                                && t.chars().filter(|c| c.is_ascii_digit()).count() >= 2
                                && t.chars().any(|c| "+-−×÷*/=:".contains(c))
                                && !crate::ocr::to_tokens(t).is_empty()
                                && others.len() < 6
                            {
                                others.push((t.clone(), lines[j].bbox));
                            }
                        }
                        PhotoState::Done {
                            bbox: Some(lines[i].bbox),
                            text: texts[i].clone(),
                            tokens,
                            img: thumb(&img, 900),
                            others,
                        }
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
            let (l, r) = match pos {
                Some(p) => (&tokens[..p], &tokens[p + 1..]),
                None => (tokens, &[][..]),
            };
            if let (Ok(lp), Ok(rp)) = (ev.eval_poly(l), ev.eval_poly(r)) {
                let [c, b, a] = [lp.0[0] - rp.0[0], lp.0[1] - rp.0[1], lp.0[2] - rp.0[2]];
                if a.abs() < 1e-12 && b.abs() > 1e-12 {
                    steps.push(crate::i18n::tf(
                        "Move terms: {}x = {}",
                        &[&display_num(b), &display_num(-c)],
                    ));
                    steps.push(crate::i18n::tf(
                        "Divide by {}: x = {} ÷ {}",
                        &[&display_num(b), &display_num(-c), &display_num(b)],
                    ));
                } else if a.abs() > 1e-12 {
                    steps.push(crate::i18n::tf(
                        "Quadratic equation: a = {}, b = {}, c = {}",
                        &[&display_num(a), &display_num(b), &display_num(c)],
                    ));
                    steps.push(format!(
                        "D = b² − 4ac = {}",
                        display_num(b * b - 4.0 * a * c)
                    ));
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
            match ev.eval(tokens) {
                Ok(v) => (format!("= {}", display_num(v)), vec![]),
                Err(e) => (expr::error_text(&e).to_string(), vec![]),
            }
        }
    }
}

pub fn game_names() -> &'static [&'static str; crate::games::GAME_COUNT] { &extra_pages::GAME_NAMES }

pub fn thumb(img: &image::RgbImage, max: u32) -> image::RgbImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= max {
        return img.clone();
    }
    image::imageops::thumbnail(
        img,
        (w as u64 * max as u64 / w.max(h) as u64) as u32,
        (h as u64 * max as u64 / w.max(h) as u64) as u32,
    )
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

pub struct Theme {
    pub bg: Rgb,
    pub key: Rgb,
    pub key_op: Rgb,
    pub key_eq: Rgb,
    pub key_eq_text: Rgb,
    pub key_text: Rgb,
    pub text: Rgb,
    pub dim: Rgb,
    pub chip: Rgb,
    pub chip_on: Rgb,
    pub chip_on_text: Rgb,
    pub pressed: Rgb,
    pub line: Rgb,
    pub shadow: Rgb,
}

pub static LOOK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub static AMOLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static CONTRAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static FONT_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1000);
pub fn font_scale() -> f32 {
    FONT_SCALE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 1000.0
}

pub fn theme(dark: bool) -> Theme {
    let mut th = base_theme(dark);
    if dark && AMOLED.load(std::sync::atomic::Ordering::Relaxed) {
        // AMOLED black: pure black background, near-black surfaces
        th.bg = [0, 0, 0];
        th.chip = [0x10, 0x10, 0x14];
        th.line = [0x22, 0x22, 0x28];
        th.shadow = [0, 0, 0];
    }
    if CONTRAST.load(std::sync::atomic::Ordering::Relaxed) {
        // High contrast: pure black/white surfaces with neutral grey keys. The accent keeps its own hue
        // (made lighter on black, darker on white) instead of turning everything yellow.
        let v = LOOK.load(std::sync::atomic::Ordering::Relaxed);
        let base: Rgb = if v != 0 {
            [(v >> 16) as u8, (v >> 8) as u8, v as u8]
        } else {
            th.key_eq
        };
        let acc = contrast_accent(base, dark);
        let acc_text: Rgb = if dark { [0, 0, 0] } else { [0xFF, 0xFF, 0xFF] };
        return if dark {
            Theme {
                bg: [0, 0, 0],
                key: [0x14, 0x14, 0x14],
                key_op: [0x38, 0x38, 0x38],
                key_eq: acc,
                key_eq_text: acc_text,
                key_text: [0xFF, 0xFF, 0xFF],
                text: [0xFF, 0xFF, 0xFF],
                dim: [0xE0, 0xE0, 0xE0],
                chip: [0x2A, 0x2A, 0x2A],
                chip_on: acc,
                chip_on_text: acc_text,
                pressed: [0x70, 0x70, 0x70],
                line: [0xFF, 0xFF, 0xFF],
                shadow: [0, 0, 0],
            }
        } else {
            Theme {
                bg: [0xFF, 0xFF, 0xFF],
                key: [0xFF, 0xFF, 0xFF],
                key_op: [0xD6, 0xD6, 0xD6],
                key_eq: acc,
                key_eq_text: acc_text,
                key_text: [0, 0, 0],
                text: [0, 0, 0],
                dim: [0x20, 0x20, 0x20],
                chip: [0xDD, 0xDD, 0xDD],
                chip_on: acc,
                chip_on_text: acc_text,
                pressed: [0x99, 0x99, 0x99],
                line: [0, 0, 0],
                shadow: [0, 0, 0],
            }
        };
    }
    let v = LOOK.load(std::sync::atomic::Ordering::Relaxed);
    if v != 0 {
        let c: Rgb = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        let mix = |a: Rgb, b: Rgb, f: f32| -> Rgb {
            [0, 1, 2].map(|i| (a[i] as f32 * (1.0 - f) + b[i] as f32 * f) as u8)
        };
        th.key_eq = c;
        th.key_eq_text = [0xFF, 0xFF, 0xFF];
        th.chip_on = c;
        th.chip_on_text = [0xFF, 0xFF, 0xFF];
        th.key_op = mix(th.key_op, c, if dark { 0.30 } else { 0.22 });
    }
    th
}

fn luminance(c: Rgb) -> f32 {
    let f = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])
}

/// Lightens (dark screens) or darkens (light screens) a color until it has at least 7:1 contrast.
pub fn contrast_accent(c: Rgb, dark: bool) -> Rgb {
    let mut c = c;
    for _ in 0..20 {
        let l = luminance(c);
        if (dark && l >= 0.30) || (!dark && l <= 0.10) {
            break;
        }
        let target = if dark { 255.0 } else { 0.0 };
        c = [0, 1, 2].map(|i| (c[i] as f32 * 0.85 + target * 0.15) as u8);
    }
    c
}

fn base_theme(dark: bool) -> Theme {
    if dark {
        Theme {
            bg: [0x0E, 0x0F, 0x12],
            key: [0x1D, 0x1F, 0x24],
            key_op: [0x2B, 0x28, 0x34],
            key_eq: [0x8D, 0x7C, 0xFF],
            key_eq_text: [0x13, 0x12, 0x18],
            key_text: [0xF2, 0xF0, 0xF7],
            text: [0xFF, 0xFF, 0xFF],
            dim: [0xA9, 0xA6, 0xB3],
            chip: [0x1A, 0x1B, 0x20],
            chip_on: [0x8D, 0x7C, 0xFF],
            chip_on_text: [0x13, 0x12, 0x18],
            pressed: [0x3A, 0x3B, 0x44],
            line: [0x2B, 0x2C, 0x33],
            shadow: [0x05, 0x06, 0x08],
        }
    } else {
        Theme {
            bg: [0xF7, 0xF7, 0xFA],
            key: [0xFF, 0xFF, 0xFF],
            key_op: [0xEB, 0xE8, 0xF7],
            key_eq: [0x5F, 0x55, 0xD9],
            key_eq_text: [0xFF, 0xFF, 0xFF],
            key_text: [0x1E, 0x1E, 0x23],
            text: [0x1B, 0x1B, 0x1F],
            dim: [0x6F, 0x6F, 0x78],
            chip: [0xEA, 0xE9, 0xF0],
            chip_on: [0x5F, 0x55, 0xD9],
            chip_on_text: [0xFF, 0xFF, 0xFF],
            pressed: [0xD7, 0xD5, 0xDF],
            line: [0xE2, 0xE0, 0xE8],
            shadow: [0xC4, 0xC1, 0xCB],
        }
    }
}

// ---------------- widgets ----------------

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Digit,
    Op,
    Eq,
    Sci,
    Flat,
    Chip,
    ChipOn,
    Row,
    Invisible,
}

pub enum W {
    Fill {
        r: Rect,
        c: Rgb,
        radius: f32,
    },
    Text {
        r: Rect,
        text: String,
        px: f32,
        bold: bool,
        c: Rgb,
        align: u8,
        wrap: bool,
    },
    Button {
        r: Rect,
        label: String,
        px: f32,
        kind: Kind,
        action: Action,
    },
    Photo {
        r: Rect,
    },
    Thumb {
        r: Rect,
        idx: usize,
    },
    /// Polylines in screen coordinates (graphs); color index 0 = text, 1 = accent, 2 = dim grid.
    Plot {
        r: Rect,
        lines: Vec<(u8, Vec<(f32, f32)>)>,
    },
}

pub struct Frame {
    pub widgets: Vec<W>,
    pub scroll_area: Option<Rect>,
    pub page: (usize, usize),
}


fn shift_xy(w: &mut W, dx: f32, dy: f32) {
    match w {
        W::Fill { r, .. }
        | W::Text { r, .. }
        | W::Button { r, .. }
        | W::Photo { r }
        | W::Thumb { r, .. } => {
            r.x += dx;
            r.y += dy;
        }
        W::Plot { r, lines } => {
            r.x += dx;
            r.y += dy;
            for (_, l) in lines {
                for p in l {
                    p.0 += dx;
                    p.1 += dy;
                }
            }
        }
    }
}

fn shift(w: &mut W, dy: f32) {
    match w {
        W::Fill { r, .. }
        | W::Text { r, .. }
        | W::Button { r, .. }
        | W::Photo { r }
        | W::Thumb { r, .. } => r.y += dy,
        W::Plot { r, lines } => {
            r.y += dy;
            for (_, l) in lines {
                for p in l {
                    p.1 += dy;
                }
            }
        }
    }
}

impl Frame {
    pub fn hit(&self, x: f32, y: f32) -> Option<usize> {
        // topmost button wins
        for (i, w) in self.widgets.iter().enumerate().rev() {
            if let W::Button { r, .. } = w {
                if r.contains(x, y) {
                    if let Some(a) = self.scroll_area {
                        if i >= self.page.0 && i < self.page.1 && !a.contains(x, y) {
                            continue;
                        }
                    }
                    return Some(i);
                }
            }
        }
        None
    }
    pub fn action(&self, i: usize) -> Option<Action> {
        match self.widgets.get(i) {
            Some(W::Button { action, .. }) => Some(action.clone()),
            _ => None,
        }
    }
}


const GROUPSP: [char; 4] = ['\u{202F}', ' ', '\'', '\u{A0}'];

fn poly_text5(c: &[f64]) -> String {
    let mut parts: Vec<String> = vec![];
    for i in (0..c.len()).rev() {
        let v = c[i];
        if v.abs() < 1e-12 {
            continue;
        }
        let sup = ["", "", "²", "³", "⁴"][i];
        let mag = v.abs();
        let coef = if (mag - 1.0).abs() < 1e-12 && i > 0 {
            String::new()
        } else {
            expr::to_fraction(mag)
                .filter(|f| f.1 > 1 && f.1 <= 12)
                .map(|f| format!("{}/{}·", f.0, f.1))
                .unwrap_or_else(|| display_num(mag))
        };
        let term = match i {
            0 => coef,
            1 => format!("{coef}x"),
            _ => format!("{coef}x{sup}"),
        };
        if parts.is_empty() {
            parts.push(if v < 0.0 { format!("−{term}") } else { term });
        } else {
            parts.push(format!("{} {term}", if v < 0.0 { "−" } else { "+" }));
        }
    }
    if parts.is_empty() {
        "0".into()
    } else {
        parts.join(" ")
    }
}
fn poly_text(p: &expr::Poly) -> String {
    poly_text5(&p.0)
}

pub const WHATS_NEW: &str = "• Games: 2048, 15-puzzle, Tetris, Snake and Minesweeper (Tools → Games)\n• Programmer mode: hex, octal, binary and bitwise operations\n• Graph up to 3 functions together, with intersection points\n• Quiz: new physics and geometry modes, streaks and records\n• Photo solver now shows step-by-step solutions\n• Formula cheat sheet in Lithuanian and English\n• Weighted grade average tool\n• Fresh look: rounded keys, new display card, custom theme color\n• Lots of small fixes and a smoother interface\n\nPrevious update:\n• Back returns to the screen you came from\n• Tap the expression to edit individual digits at the cursor\n• Drag market charts to inspect recorded prices and times\n• More market data: cap, volume, high/low where available\n• Quiz: arithmetic, equations and chemistry, scoring\n• Save your own formulas with named variables\n• Statistics: bar and pie charts\n• Share calculations as a PNG image";

impl App {
    /// Offline or model result for the word-problem screen.
    pub fn ask_result(&mut self, r: Option<(String, bool)>) {
        match r.and_then(|(e, nano)| expr::eval_text(&e).map(|v| (e, v, nano))) {
            Some((e, v, nano)) => {
                self.ask_status.clear();
                self.ask_out = Some((
                    e.replace('*', "×").replace('/', "÷"),
                    expr::display_num(v),
                    nano,
                ));
            }
            None => {
                self.ask_out = None;
                self.ask_status = crate::i18n::t("Could not understand the problem. Try writing it with numbers, e.g. \"12 * 3 + 4\".").into();
            }
        }
    }
}

/// "13.10" or "13.10 22:05" from unix seconds (UTC).
pub fn fmt_ts(ts: f64, with_time: bool) -> String {
    let secs = ts as i64;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    if with_time {
        format!("{:02}.{:02} {:02}:{:02}", day, month, rem / 3600, rem % 3600 / 60)
    } else {
        format!("{:04}-{:02}-{:02}", year, month, day)
    }
}

/// Price with sensible decimals: 2 for normal prices, more for tiny ones.
pub fn fmt_price(v: f64) -> String {
    let r = if v.abs() >= 1.0 {
        (v * 100.0).round() / 100.0
    } else {
        (v * 1e6).round() / 1e6
    };
    expr::display_num(r)
}

#[cfg(test)]
mod release26_tests {
    use super::*;
    #[test] fn back_to_origin() {
        let mut a = App {screen:Screen::Calc,..App::default()};
        a.handle(&Action::Nav(Screen::Settings)); a.handle(&Action::Set("whatsnew"));
        a.handle(&Action::Back); assert_eq!(a.screen,Screen::Settings);
        a.handle(&Action::Back); assert_eq!(a.screen,Screen::Calc);
        a.handle(&Action::Nav(Screen::Tools)); a.handle(&Action::Nav(Screen::Periodic)); a.handle(&Action::ElementOpen(2));
        a.handle(&Action::Back); assert_eq!(a.screen,Screen::Periodic);
    }
    #[test] fn edit_inside_number() {
        let mut a=App::default(); a.press("1"); a.press("2"); a.press("3");
        a.handle(&Action::EditCursor(1)); a.press("9"); assert_eq!(a.edit_text.as_deref(),Some("1923"));
        a.press("⌫"); assert_eq!(a.edit_text.as_deref(),Some("123"));
        a.handle(&Action::EditCursor(3)); a.press("+"); a.press("4"); a.press("="); assert_eq!(a.current_value(),Some(127.0));
    }
}

#[path = "pages.rs"]
mod pages;
#[path = "actions.rs"]
mod actions;
#[path = "calculator_ui.rs"]
mod calculator_ui;
#[path = "entry.rs"]
mod entry;
#[path = "layout.rs"]
mod layout;
#[path = "advanced_pages.rs"]
mod advanced_pages;
#[path = "extra_pages.rs"]
mod extra_pages;
#[path = "render.rs"]
mod render;
pub use render::{draw,text_in,PHOTO_SRC_W};
