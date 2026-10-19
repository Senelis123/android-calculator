use super::*;
impl App {
    pub fn handle(&mut self, a: &Action) -> Vec<Effect> {
        let mut fx = vec![];
        let save_before = (
            self.s.mem[self.s.mem_slot],
            self.s.history.len(),
            self.s.theme,
            self.s.vibrate,
            self.s.degrees,
            self.s.sci,
        );
        self.popup = matches!(a, Action::DisplayLongPress);
        match a {
            Action::Key(k) => {
                if self.s.vibrate {
                    fx.push(Effect::Vibrate);
                }
                if self.s.sound {
                    fx.push(Effect::Click);
                }
                if k.starts_with('M') {
                    self.memory(k);
                } else if k == "DEG" || k == "RAD" {
                    self.s.degrees = !self.s.degrees;
                } else {
                    self.press(k);
                    if k == "=" && self.s.speak {
                        if let Some(v) = self.current_value() {
                            fx.push(Effect::Speak(
                                expr::words(v, crate::i18n::lang())
                                    .unwrap_or_else(|| expr::display_num(v)),
                            ));
                        }
                    }
                }
            }
            Action::Nav(s) => {
                self.navigate(*s);
                self.scroll = 0.0;
                if *s == Screen::Ask && self.nano.is_empty() {
                    fx.push(Effect::NanoStatus);
                }
                if *s == Screen::Graph {
                    let trig = self
                        .tokens
                        .iter()
                        .any(|t| matches!(t, Tok::Func("sin" | "cos" | "tan")));
                    self.graph_span = if trig && self.s.degrees { 360.0 } else { 10.0 };
                }
                if *s == Screen::Steps {
                    self.steps_src = None;
                }
                if *s == Screen::Converter && self.s.conv_cat == CURRENCY_CAT {
                    fx.push(Effect::FetchRates);
                }
                if *s == Screen::Markets {
                    fx.push(Effect::FetchMarkets(self.market_kind));
                }
            }
            Action::Back => {
                self.scroll = 0.0;
                self.screen = self.nav_stack.pop().unwrap_or(Screen::Calc);
                self.text_target = None;
                fx.push(Effect::Keyboard(false));
            }
            Action::UseValue(v) => {
                self.set_value(*v);
                self.screen = Screen::Calc;
            }
            Action::UseTokens => {
                if let PhotoState::Done { tokens, .. } = &self.photo {
                    self.tokens = tokens.clone();
                    self.just_evaluated = false;
                    self.result = None;
                }
                self.screen = Screen::Calc;
            }
            Action::LoadHistory(i) => {
                if let Some(h) = self.s.history.get(*i) {
                    if let Some(v) = h.value {
                        self.set_value(v);
                    }
                }
                self.screen = Screen::Calc;
            }
            Action::ClearHistory => self.s.history.retain(|h| h.pinned),
            Action::Pin(i) => {
                if let Some(h) = self.s.history.get_mut(*i) {
                    h.pinned = !h.pinned;
                }
                fx.push(Effect::Save);
            }
            Action::DeleteHistory(i) => {
                if *i < self.s.history.len() {
                    self.s.history.remove(*i);
                    self.toast = Some(crate::i18n::t("Deleted").into());
                }
                fx.push(Effect::Save);
            }
            Action::Swipe(right, on) => match (self.screen, &**on) {
                (Screen::History, Action::LoadHistory(i)) => {
                    let i = *i;
                    if i < self.s.history.len() {
                        self.s.history.remove(i);
                        self.toast = Some(crate::i18n::t("Deleted").into());
                        fx.push(Effect::Save);
                    }
                }
                (Screen::Calc, _) => {
                    self.press(if *right { "↶" } else { "⌫" });
                    if self.s.vibrate {
                        fx.push(Effect::Vibrate);
                    }
                }
                _ => {}
            },
            Action::StartText(k, i) => {
                self.text_target = Some((*k, *i));
                fx.push(Effect::Keyboard(true));
            }
            Action::EndText => {
                self.text_target = None;
                fx.push(Effect::Keyboard(false));
                fx.push(Effect::Save);
            }
            Action::TextKey(c) => match self.text_target {
                Some((0, _)) => self.search.push(*c),
                Some((2, _)) => {
                    if self.ask_text.chars().count() < 500 {
                        self.ask_text.push(*c)
                    }
                }
                Some((1, i)) => {
                    if let Some(h) = self.s.history.get_mut(i) {
                        if h.note.chars().count() < 200 {
                            h.note.push(*c);
                        }
                    }
                }
                _ => {}
            },
            Action::TextBack => match self.text_target {
                Some((0, _)) => {
                    self.search.pop();
                }
                Some((2, _)) => {
                    self.ask_text.pop();
                }
                Some((1, i)) => {
                    if let Some(h) = self.s.history.get_mut(i) {
                        h.note.pop();
                    }
                }
                _ => {}
            },
            Action::MarketKind(k) => {
                self.market_kind = *k;
                self.market_status = crate::i18n::t("Loading market data…").into();
                fx.push(Effect::FetchMarkets(*k));
            }
            Action::MarketOpen(i) => {
                if let Some(q) = self.market_quotes.get(*i).cloned() {
                    self.market_sel = Some(q);
                    self.market_hist.clear();
                    self.navigate(Screen::MarketDetail);
                    self.scroll = 0.0;
                    self.market_hist_status = crate::i18n::t("Loading…").into();
                    fx.push(Effect::FetchMarketHistory(
                        self.market_kind,
                        self.market_sel.as_ref().map(|q| q.id.clone()).unwrap_or_default(),
                        self.market_range,
                    ));
                }
            }
            Action::GameNav(k) => {
                let kind = (*k).min(crate::games::GAME_COUNT - 1);
                if self.game.is_none() || self.game_kind != kind {
                    self.new_game(kind);
                }
                self.navigate(Screen::Game(kind));
                self.scroll = 0.0;
            }
            Action::GameNew => {
                self.new_game(self.game_kind);
            }
            Action::GameMove(c) => self.game_move(*c, &mut fx),
            Action::GameTap(i) => self.game_tap(*i, &mut fx),
            Action::Tick => self.game_tick(&mut fx),
            Action::ProgKey(k) => {
                self.prog.key(k);
            }
            Action::QuizMode(mode) => {
                self.quiz_mode=(*mode).min(4);self.quiz_count=0;self.quiz_score=0;self.next_question();
            }
            Action::QuizAnswer(i) => {
                if !self.quiz_answered {
                    if let Some(q)=&self.quiz_question {
                        let ok=*i==q.answer;
                        if ok {self.quiz_score+=1;self.quiz_streak+=1;}
                        else {self.quiz_streak=0;}
                        self.quiz_count+=1;self.quiz_answered=true;
                        let prefix=if ok {crate::i18n::t("Correct!")} else {crate::i18n::t("Not quite.")};
                        self.quiz_feedback=format!("{} {}",prefix,q.explanation);
                        let score=self.quiz_score;let mode=self.quiz_mode;
                        self.quiz_record(mode,score);
                        self.s.quiz_streak_best=self.s.quiz_streak_best.max(self.quiz_streak);
                        fx.push(Effect::Save);
                    }
                }
            }
            Action::FormulaOpen(i) => {
                if let Some(f)=self.s.formulas.get(*i) {
                    self.formula_sel=Some(*i);
                    let vars=f.variables();
                    if vars.is_empty() {self.formula_result=f.evaluate(&[]).map(display_num).unwrap_or_else(|e|e);}
                    else {fx.push(Effect::InputDialog(format!("{}: values",f.name),vars.iter().map(|v|format!("{v}=0")).collect::<Vec<_>>().join("\n"),4));}
                }
            }
            Action::FormulaDelete(i) => {if *i<self.s.formulas.len(){self.s.formulas.remove(*i);self.formula_sel=None;self.formula_result.clear();fx.push(Effect::Save);}}
            Action::Set("quiz_next") => {if self.quiz_count>=10 {self.quiz_count=0;self.quiz_score=0;}self.next_question();}
            Action::Set("formula_new") => fx.push(Effect::InputDialog("Name on first line, formula on second line".into(),"Rectangle area\nwidth*height".into(),3)),
            Action::Set("stats_input") => fx.push(Effect::InputDialog("Data: numbers separated by spaces (decimal point)".into(),self.stats_data.iter().map(|v|v.to_string()).collect::<Vec<_>>().join(" "),5)),
            Action::Set("stats_pie") => self.stats_pie = !self.stats_pie,
            Action::Set("graph_fns") => {
                let cur = self.graph_toks.iter().map(|t| display_tokens(t)).collect::<Vec<_>>().join("\n");
                let hint = if cur.is_empty() { "x^2 - 2\nsin(x)".into() } else { cur };
                fx.push(Effect::InputDialog(crate::i18n::t("Up to 3 functions of x, one per line").into(), hint, 6));
            }
            Action::Set("graph_clear") => self.graph_toks.clear(),
            Action::Set("photo_steps") => {
                if let PhotoState::Done { tokens, .. } = &self.photo {
                    self.steps_src = Some(tokens.clone());
                    self.navigate(Screen::Steps);
                    self.scroll = 0.0;
                }
            }
            Action::Set("mines_flag") => self.mines_flag = !self.mines_flag,
            Action::Set("prog_b2") => self.prog.set_base(2),
            Action::Set("prog_b8") => self.prog.set_base(8),
            Action::Set("prog_b10") => self.prog.set_base(10),
            Action::Set("prog_b16") => self.prog.set_base(16),
            Action::Set("accent_custom") => fx.push(Effect::InputDialog(crate::i18n::t("Accent color as #RRGGBB (for example #1E6FD9)").into(),"#1E6FD9".into(),7)),
            Action::ShareHistoryImage(i) => {
                if let Some(h)=self.s.history.get(*i) {fx.push(Effect::ShareImage(h.expr.clone(),h.result.clone()));}
            }
            Action::Set("share_image") => {
                let (line,result)=self.result.clone().unwrap_or_else(||(display_tokens(&self.tokens),self.current_value().map(display_num).unwrap_or_default()));
                fx.push(Effect::ShareImage(line,result));
            }
            Action::EditCursor(c) => {
                if self.edit_text.is_none() { self.edit_text = Some(expr::editable_text(&self.tokens)); }
                self.edit_cursor = (*c).min(self.edit_text.as_ref().unwrap().chars().count());
                self.result = None; self.just_evaluated = false;
            }
            Action::MarketPoint(i) => self.market_point = (*i).min(self.market_hist.len().saturating_sub(1)),
            Action::MarketRange(r) => {
                self.market_point = 0;
                self.market_range = (*r).min(crate::market::RANGES.len() - 1);
                if let Some(q) = &self.market_sel {
                    self.market_hist.clear();
                    self.market_hist_status = crate::i18n::t("Loading…").into();
                    fx.push(Effect::FetchMarketHistory(
                        self.market_kind,
                        q.id.clone(),
                        self.market_range,
                    ));
                }
            }
            Action::ElementOpen(i) => {
                self.elem_sel = (*i).min(crate::science::ELEMENTS.len() - 1);
                self.navigate(Screen::Element);
                self.scroll = 0.0;
            }
            Action::Surface(i) => self.g3_surface = *i,
            Action::Set("rot_l") => self.g3_yaw -= 0.3,
            Action::Set("rot_r") => self.g3_yaw += 0.3,
            Action::Set("rot_u") => self.g3_pitch = (self.g3_pitch + 0.15).min(1.5),
            Action::Set("rot_d") => self.g3_pitch = (self.g3_pitch - 0.15).max(0.0),
            Action::Set("g3zoom+") => self.g3_span = (self.g3_span * 0.7).max(0.5),
            Action::Set("g3zoom-") => self.g3_span = (self.g3_span / 0.7).min(40.0),
            Action::MarketRefresh => {
                self.market_status = crate::i18n::t("Refreshing market data…").into();
                fx.push(Effect::FetchMarkets(self.market_kind));
            }
            Action::HwKey(c) => {
                // hardware keyboard on the calculator screen
                let k: Option<&str> = match c {
                    '0'..='9' => None,
                    '+' => Some("+"),
                    '-' => Some("−"),
                    '*' => Some("×"),
                    '/' => Some("÷"),
                    '^' => Some("xʸ"),
                    '(' => Some("("),
                    ')' => Some(")"),
                    '.' | ',' => Some(","),
                    '=' | '\n' => Some("="),
                    '%' => Some("%"),
                    '!' => Some("n!"),
                    'x' => Some("x"),
                    '\u{8}' => Some("⌫"),
                    '\u{1b}' => Some("C"),
                    _ => Option::None,
                };
                if c.is_ascii_digit() {
                    let s = c.to_string();
                    self.press(&s);
                } else if let Some(k) = k {
                    self.press(k);
                }
            }
            Action::ToggleSci => self.s.sci = !self.s.sci,
            Action::CycleTheme => self.s.theme = (self.s.theme + 1) % 4,
            Action::CycleLang => {
                let n = crate::i18n::LANGS.len() as i32;
                self.s.lang = if self.s.lang + 1 >= n {
                    -1
                } else {
                    self.s.lang + 1
                };
                self.apply_lang();
            }
            Action::ToggleVibrate => self.s.vibrate = !self.s.vibrate,
            Action::ToggleAngle => self.s.degrees = !self.s.degrees,
            Action::CheckUpdate => {
                fx.push(Effect::CheckUpdate);
                self.toast = Some(crate::i18n::t("Checking for updates…").into());
            }
            Action::UpdateBanner => fx.push(Effect::UpdateTap),
            Action::Copy => {
                let text = match self.current_value() {
                    Some(v) => expr::clipboard_num(v),
                    None => self
                        .result
                        .as_ref()
                        .map(|r| r.1.clone())
                        .unwrap_or_default(),
                };
                if !text.is_empty() {
                    fx.push(Effect::Copy(text));
                    self.toast = Some(crate::i18n::t("Copied").into());
                }
            }
            Action::Paste => fx.push(Effect::RequestPaste),
            Action::DisplayLongPress | Action::ClosePopup | Action::None => {}
            Action::ConvCat(c) => {
                self.s.conv_cat = *c;
                if *c == CURRENCY_CAT {
                    fx.push(Effect::FetchRates);
                }
                fx.push(Effect::Save);
            }
            Action::PickUnit(i) => {
                let c = self.s.conv_cat;
                if let Screen::PickUnit { from } = self.screen {
                    if from {
                        self.s.conv_from[c] = *i;
                    } else {
                        self.s.conv_to[c] = *i;
                    }
                }
                self.screen = Screen::Converter;
                fx.push(Effect::Save);
            }
            Action::SwapUnits => {
                let c = self.s.conv_cat;
                std::mem::swap(&mut self.s.conv_from[c], &mut self.s.conv_to[c]);
                fx.push(Effect::Save);
            }
            Action::RefreshRates => fx.push(Effect::FetchRates),
            Action::Tip(i) => {
                self.s.tip = *i;
                fx.push(Effect::Save);
            }
            Action::People(n) => {
                self.s.people = *n;
                fx.push(Effect::Save);
            }
            Action::TakePhoto => fx.push(Effect::TakePhoto),
            Action::OpenGallery => {
                self.navigate(Screen::Gallery);
                self.scroll = 0.0;
                fx.push(Effect::OpenGallery);
            }
            Action::Set("gzoom+") => self.graph_span = (self.graph_span / 2.0).max(0.01),
            Action::Set("gzoom-") => self.graph_span = (self.graph_span * 2.0).min(1e6),
            Action::Set("fav") => {
                let c = self.s.conv_cat;
                if let Some(p) = self.s.fav_cats.iter().position(|x| *x == c) {
                    self.s.fav_cats.remove(p);
                } else {
                    self.s.fav_cats.push(c);
                }
                fx.push(Effect::Save);
            }
            Action::Set("ratehist") => {
                self.rate_hist = None;
                fx.push(Effect::FetchRateHistory);
            }
            Action::Set("export") => {
                let mut csv = String::from("expression;result;note\n");
                for h in &self.s.history {
                    csv.push_str(&format!(
                        "\"{}\";\"{}\";\"{}\"\n",
                        h.expr.replace('"', "'"),
                        h.result.replace('"', "'"),
                        h.note.replace('"', "'")
                    ));
                }
                fx.push(Effect::SaveFile(
                    "calculator-history.csv".into(),
                    csv.clone(),
                ));
                fx.push(Effect::Share(csv));
            }
            Action::Set("backup") => {
                let mut b = self.s.clone();
                b.rates = None;
                if let Ok(js) = serde_json::to_string(&b) {
                    fx.push(Effect::Copy(js.clone()));
                    fx.push(Effect::SaveFile("calculator-backup.json".into(), js));
                    self.toast =
                        Some(crate::i18n::t("Backup copied and saved to Downloads").into());
                }
            }
            Action::Set("restore") => {
                self.restore_pending = true;
                fx.push(Effect::RequestPaste);
            }
            Action::Set("whatsnew") => {
                self.navigate(Screen::WhatsNew);
                self.scroll = 0.0;
            }
            Action::PhotoPick(i) => {
                if let PhotoState::Done {
                    bbox,
                    text,
                    tokens,
                    others,
                    ..
                } = &mut self.photo
                {
                    if *i < others.len() {
                        let (t2, b2) = others.remove(*i);
                        if let Some(b) = bbox.take() {
                            others.insert(0, (text.clone(), b));
                        }
                        *tokens = crate::ocr::to_tokens(&t2);
                        *text = t2;
                        *bbox = Some(b2);
                    }
                }
            }
            Action::Set("photo_share") => {
                if let PhotoState::Done { tokens, .. } = &self.photo {
                    let (ans, steps) = self.photo_answer(tokens);
                    fx.push(Effect::Share(format!(
                        "{}\n{}\n{}",
                        display_tokens(tokens),
                        steps.join("\n"),
                        ans
                    )));
                }
            }
            Action::Set("photo_words") => {
                if let PhotoState::Done { text, .. } = &self.photo {
                    self.ask_text = text.clone();
                    self.ask_out = None;
                    self.screen = Screen::Ask;
                    fx.push(Effect::Solve(
                        self.ask_text.clone(),
                        self.nano == "available",
                    ));
                    if self.nano.is_empty() {
                        fx.push(Effect::NanoStatus);
                    }
                }
            }
            Action::Set("ask_voice") => {
                self.text_target = None;
                fx.push(Effect::Keyboard(false));
                fx.push(Effect::Listen);
            }
            Action::Set("ask_paste") => fx.push(Effect::RequestPaste),
            Action::Set("ask_clear") => {
                self.ask_text.clear();
                self.ask_out = None;
                self.ask_status.clear();
            }
            Action::Set("ask_solve") => {
                self.text_target = None;
                fx.push(Effect::Keyboard(false));
                if self.ask_text.trim().is_empty() {
                    self.ask_status = crate::i18n::t("Type or say a problem first.").into();
                } else {
                    self.ask_status = crate::i18n::t("Solving…").into();
                    self.ask_out = None;
                    fx.push(Effect::Solve(
                        self.ask_text.clone(),
                        self.nano == "available",
                    ));
                }
            }
            Action::Set("ask_open") => {
                if let Some((e, _, _)) = &self.ask_out {
                    if let Some(t) = expr::tokenize(e) {
                        self.tokens = t;
                        self.just_evaluated = false;
                        self.result = None;
                        self.screen = Screen::Calc;
                        self.press("=");
                    }
                }
            }
            Action::Set("closenew") => {
                self.whats_new = false;
                self.screen = self.nav_stack.pop().unwrap_or(Screen::Calc);
                self.scroll = 0.0;
            }
            Action::Set("crash_close") => self.crash = None,
            Action::Set("crash_copy") => {
                if let Some(c) = &self.crash {
                    fx.push(Effect::Copy(format!("Calculator {}\n{}", version(), c)));
                    self.toast = Some(crate::i18n::t("Copied").into());
                }
            }
            Action::Set("crash_share") => {
                if let Some(c) = &self.crash {
                    fx.push(Effect::Share(format!("Calculator {}\n{}", version(), c)));
                }
            }
            Action::Set("bug") => fx.push(Effect::ReportBug),
            Action::Set("floating") => fx.push(Effect::Floating),
            Action::Set("bgcheck") => {
                self.s.bg_check = !self.s.bg_check;
                fx.push(Effect::BgCheck(self.s.bg_check));
                fx.push(Effect::Save);
            }
            Action::Set("speaknow") => {
                if let Some(v) = self.current_value() {
                    fx.push(Effect::Speak(
                        expr::words(v, crate::i18n::lang()).unwrap_or_else(|| expr::display_num(v)),
                    ));
                }
            }
            Action::Set("icon") => {
                self.cycle_setting("icon");
                fx.push(Effect::SetIcon(self.s.icon));
                fx.push(Effect::Save);
            }
            Action::Set("fraction") => {
                self.show_frac = !self.show_frac;
            }
            Action::Set(k) => {
                self.cycle_setting(k);
                fx.push(Effect::Save);
            }
            Action::OpenTool(i) => {
                self.tool = *i;
                self.tool_mode = 0;
                self.list.clear();
                self.focus = 0;
                self.fields = vec![String::new(); crate::tools::TOOLS[*i].fields.len()];
                // sensible prefills: calculator value into the first field, today into dates
                if let Some(v) = self.current_value().filter(|v| *v != 0.0) {
                    if *i != crate::tools::T_DATES && *i != crate::tools::T_RANDOM {
                        self.fields[0] = plain(v);
                    }
                }
                if *i == crate::tools::T_DATES {
                    let (y, m, d) = crate::tools::civil_from_days(crate::tools::today_days());
                    self.fields = vec![
                        y.to_string(),
                        m.to_string(),
                        d.to_string(),
                        y.to_string(),
                        m.to_string(),
                        d.to_string(),
                    ];
                }
                if *i == crate::tools::T_RANDOM {
                    self.fields = vec!["1".into(), "100".into()];
                }
                if *i == 4 {
                    self.fields[1] = "0".into();
                }
                if *i == 5 {
                    self.fields[3] = "1".into();
                }
                self.navigate(Screen::Tool);
                self.scroll = 0.0;
            }
            Action::ToolMode(m) => {
                self.tool_mode = *m;
                let vis = crate::tools::visible_fields(self.tool, *m);
                if !vis.contains(&self.focus) {
                    self.focus = vis[0];
                }
                self.reroll();
            }
            Action::Focus(f) => self.focus = *f,
            Action::FormKey(k) => {
                if self.s.vibrate {
                    fx.push(Effect::Vibrate);
                }
                self.form_key(k);
            }
            Action::GalleryPick(id) => {
                self.screen = Screen::Photo;
                self.photo = PhotoState::Working;
                fx.push(Effect::LoadGallery(*id));
            }
        }
        let after = (
            self.s.mem[self.s.mem_slot],
            self.s.history.len(),
            self.s.theme,
            self.s.vibrate,
            self.s.degrees,
            self.s.sci,
        );
        if after != save_before && !fx.contains(&Effect::Save) {
            fx.push(Effect::Save);
        }
        fx
    }

    pub(super) fn reroll(&mut self) {
        let mut x = self.seed ^ (self.seed << 13);
        x ^= x >> 7;
        x ^= x << 17;
        self.seed = x;
    }

}
