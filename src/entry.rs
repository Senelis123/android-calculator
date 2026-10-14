use super::*;
impl App {
    pub(super) fn open_parens(&self) -> i32 {
        self.tokens
            .iter()
            .map(|t| match t {
                Tok::LParen | Tok::Func(_) => 1,
                Tok::RParen => -1,
                _ => 0,
            })
            .sum()
    }

    pub(super) fn start_fresh_if_evaluated(&mut self, keep_value: bool) {
        if self.just_evaluated {
            if !keep_value {
                self.tokens.clear();
            }
            self.just_evaluated = false;
            self.result = None;
        }
    }

    pub fn set_value(&mut self, v: f64) {
        self.edit_text = None;
        self.tokens = vec![Tok::Num(plain(v))];
        self.just_evaluated = true;
        self.result = None;
    }

    pub(super) fn push_value_tok(&mut self, t: Tok) {
        self.start_fresh_if_evaluated(false);
        self.tokens.push(t);
    }

    pub fn press(&mut self, key: &str) {
        if let Some(mut text) = self.edit_text.take() {
            let before=self.tokens.clone();
            let mut chars: Vec<char> = text.chars().collect();
            self.edit_cursor = self.edit_cursor.min(chars.len());
            match key {
                "◀" => self.edit_cursor = self.edit_cursor.saturating_sub(1),
                "▶" => self.edit_cursor = (self.edit_cursor + 1).min(chars.len()),
                "⌫" => { if self.edit_cursor > 0 { self.edit_cursor -= 1; chars.remove(self.edit_cursor); } },
                "C" => { chars.clear(); self.edit_cursor = 0; },
                "=" => {
                    let Some(tokens)=expr::tokenize_edit(&text) else {
                        self.toast=Some("Check the expression".into());self.edit_text=Some(text);return;
                    };
                    self.tokens = tokens;
                    if self.tokens.is_empty() && !text.is_empty() {
                        self.toast = Some("Check the expression".into()); self.edit_text = Some(text); return;
                    }
                    self.cursor = None; self.press_inner("="); return;
                },
                "↶" | "↷" => { self.cursor = None; self.press(key); return; },
                _ => {
                    let k = match key { "×" => "*", "÷" => "/", "−" => "-", "√" => "sqrt(", "±"=>"-", "xʸ"=>"^", "n!"=>"!", "x²"=>"²", "x³"=>"³", _ => key };
                    let k = crate::science::constant_value(k).unwrap_or(k);
                    let extra=crate::science::extra_func(k).is_some();
                    let k = if extra || expr::FUNCS.contains(&k) || matches!(k,"exp"|"pow10"|"pow2"|"log2"|"cbrt"|"trunc"|"sec"|"csc"|"cot"|"asinh"|"acosh"|"atanh"|"floor"|"ceil"|"round"|"sign"|"gamma") { format!("{k}(") } else { k.to_string() };
                    for c in k.chars() { chars.insert(self.edit_cursor, c); self.edit_cursor += 1; }
                }
            }
            text = chars.into_iter().collect();
            self.tokens = expr::tokenize_edit(&text).unwrap_or_else(|| self.tokens.clone());
            if self.tokens!=before {self.undo.push(before);self.redo.clear();}
            self.edit_text = Some(text); self.result = None; self.just_evaluated = false; return;
        }
        match key {
            "◀" => {
                let n = self.tokens.len();
                let c = self.cursor.unwrap_or(n);
                self.cursor = Some(c.saturating_sub(1));
                self.just_evaluated = false;
                self.result = None;
                return;
            }
            "▶" => {
                let n = self.tokens.len();
                self.cursor = match self.cursor {
                    Some(c) if c + 1 < n => Some(c + 1),
                    _ => None,
                };
                return;
            }
            "↶" => {
                if let Some(p) = self.undo.pop() {
                    self.redo.push(std::mem::replace(&mut self.tokens, p));
                    self.cursor = None;
                    self.result = None;
                    self.just_evaluated = false;
                }
                return;
            }
            "↷" => {
                if let Some(p) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.tokens, p));
                    self.cursor = None;
                    self.result = None;
                    self.just_evaluated = false;
                }
                return;
            }
            "a/b" => {
                self.show_frac = !self.show_frac;
                return;
            }
            "2nd" | "3rd" | "4th" | "5th" | "6th" | "1st" => {
                self.sci_page = (self.sci_page + 1) % crate::science::SCI_PAGES as u8;
                return;
            }
            _ => {}
        }
        let before = self.tokens.clone();
        match self.cursor {
            Some(c) if c < self.tokens.len() && key != "=" && key != "C" => {
                let after = self.tokens.split_off(c);
                let ev = self.just_evaluated;
                self.just_evaluated = false;
                self.press_inner(key);
                self.just_evaluated = ev && false;
                let nc = self.tokens.len();
                self.tokens.extend(after);
                self.cursor = Some(nc);
            }
            _ => {
                self.cursor = None;
                self.press_inner(key);
            }
        }
        if self.tokens != before {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        if key != "=" {
            self.show_frac = false;
        }
    }

    pub(super) fn press_inner(&mut self, key: &str) {
        match key {
            "C" => {
                self.tokens.clear();
                self.just_evaluated = false;
                self.result = None;
            }
            "⌫" => {
                if self.just_evaluated {
                    return;
                }
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) => {
                        n.pop();
                        if n.is_empty() || n == "-" {
                            self.tokens.pop();
                        }
                    }
                    Some(_) => {
                        self.tokens.pop();
                    }
                    None => {}
                }
            }
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                self.start_fresh_if_evaluated(false);
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) if n.len() < 20 => {
                        if n == "0" {
                            *n = key.into();
                        } else if n == "-0" {
                            *n = format!("-{key}");
                        } else {
                            n.push_str(key);
                        }
                    }
                    Some(Tok::Num(_)) => {}
                    _ => self.tokens.push(Tok::Num(key.into())),
                }
            }
            "," => {
                self.start_fresh_if_evaluated(false);
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) => {
                        if !n.contains('.') && !n.contains('e') {
                            n.push('.');
                        }
                    }
                    _ => self.tokens.push(Tok::Num("0.".into())),
                }
            }
            "+" | "−" | "×" | "÷" | "xʸ" => {
                let op = if key == "xʸ" {
                    '^'
                } else {
                    key.chars().next().unwrap()
                };
                self.start_fresh_if_evaluated(true);
                match self.tokens.last() {
                    Some(Tok::Op(prev)) => {
                        if op == '−' && matches!(prev, '×' | '÷' | '^') {
                            self.tokens.push(Tok::Op('−'));
                        } else if self.tokens.len() >= 2
                            && matches!(self.tokens[self.tokens.len() - 2], Tok::Op(_))
                        {
                            self.tokens.pop();
                            self.tokens.pop();
                            self.tokens.push(Tok::Op(op));
                        } else if self.tokens.len() == 1 {
                            if op == '−' {
                                self.tokens[0] = Tok::Op('−');
                            }
                        } else {
                            *self.tokens.last_mut().unwrap() = Tok::Op(op);
                        }
                    }
                    None | Some(Tok::LParen) | Some(Tok::Func(_)) | Some(Tok::Equals) => {
                        if op == '−' {
                            self.tokens.push(Tok::Op('−'));
                        }
                    }
                    _ => self.tokens.push(Tok::Op(op)),
                }
            }
            "±" => {
                self.start_fresh_if_evaluated(true);
                if let Some(Tok::Num(n)) = self.tokens.last_mut() {
                    if n.ends_with('e') {
                        n.push('-');
                        return;
                    }
                }
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) => {
                        if n != "0" {
                            *n = match n.strip_prefix('-') {
                                Some(r) => r.to_string(),
                                None => format!("-{n}"),
                            };
                        }
                    }
                    _ => {
                        if !is_value_end(self.tokens.last()) {
                            self.tokens.push(Tok::Op('−'));
                        }
                    }
                }
            }
            "%" | "x²" | "1/x" | "n!" => {
                let p = match key {
                    "%" => "%",
                    "x²" => "²",
                    "1/x" => "⁻¹",
                    _ => "!",
                };
                self.start_fresh_if_evaluated(true);
                if is_value_end(self.tokens.last()) {
                    self.tokens.push(Tok::Post(p));
                }
            }
            "(" => self.push_value_tok(Tok::LParen),
            ")" => {
                if !self.just_evaluated
                    && self.open_parens() > 0
                    && is_value_end(self.tokens.last())
                {
                    self.tokens.push(Tok::RParen);
                }
            }
            "π" => self.push_value_tok(Tok::Pi),
            "e" => self.push_value_tok(Tok::E),
            "x" => self.push_value_tok(Tok::X),
            "√" | "sin" | "cos" | "tan" | "ln" | "log" => {
                let f: &'static str = match key {
                    "√" => "√",
                    "sin" => "sin",
                    "cos" => "cos",
                    "tan" => "tan",
                    "ln" => "ln",
                    _ => "log",
                };
                self.push_value_tok(Tok::Func(f));
            }
            "Ans" => self.push_value_tok(Tok::Ans),
            "i" => self.push_value_tok(Tok::I),
            "EE" => {
                self.start_fresh_if_evaluated(true);
                match self.tokens.last_mut() {
                    Some(Tok::Num(n)) if !n.contains('e') => n.push('e'),
                    _ => {}
                }
            }
            ";" => {
                if self.open_parens() > 0 && is_value_end(self.tokens.last()) {
                    self.tokens.push(Tok::Sep);
                }
            }
            "mod" => {
                self.start_fresh_if_evaluated(true);
                if is_value_end(self.tokens.last()) {
                    self.tokens.push(Tok::Op('m'));
                }
            }
            "<" | ">" | "≤" | "≥" => {
                if is_value_end(self.tokens.last())
                    && !self
                        .tokens
                        .iter()
                        .any(|t| matches!(t, Tok::Equals | Tok::Cmp(_)))
                {
                    self.just_evaluated = false;
                    self.result = None;
                    self.tokens.push(Tok::Cmp(key.chars().next().unwrap()));
                }
            }
            "sin⁻¹" | "cos⁻¹" | "tan⁻¹" | "sinh" | "cosh" | "tanh" | "ⁿ√" | "logₙ" | "nCr"
            | "nPr" | "abs" => {
                let f: &'static str = match key {
                    "sin⁻¹" => "asin",
                    "cos⁻¹" => "acos",
                    "tan⁻¹" => "atan",
                    "sinh" => "sinh",
                    "cosh" => "cosh",
                    "tanh" => "tanh",
                    "ⁿ√" => "root",
                    "logₙ" => "logb",
                    "nCr" => "nCr",
                    "nPr" => "nPr",
                    _ => "abs",
                };
                self.push_value_tok(Tok::Func(f));
            }
            "=" => self.equals(),
            k => {
                if let Some(f) = crate::science::extra_func(k) {
                    self.push_value_tok(Tok::Func(f));
                } else if let Some(raw) = crate::science::constant_value(k) {
                    self.push_value_tok(Tok::Num(raw.to_string()));
                }
            }
        }
    }

    pub(super) fn equals(&mut self) {
        if self.just_evaluated || self.tokens.is_empty() {
            return;
        }
        let has_x = self.tokens.contains(&Tok::X);
        let line0 = display_tokens(&self.tokens);
        if self.tokens.iter().any(|t| matches!(t, Tok::Cmp(_))) {
            let text = match self.ev().solve_ineq(&self.tokens) {
                Ok(s) => s,
                Err(e) => solve_error_text(&e).to_string(),
            };
            self.s
                .history
                .insert(0, HistItem::new(line0.clone(), text.clone(), None));
            self.result = Some((line0, text));
            self.tokens.clear();
            self.just_evaluated = true;
            self.cursor = None;
            return;
        }
        if self.tokens.contains(&Tok::I) {
            let text = match crate::complex::eval(&self.tokens, self.s.degrees, self.s.ans) {
                Ok(z) => z.text(),
                Err(e) => expr::error_text(&e).to_string(),
            };
            self.s
                .history
                .insert(0, HistItem::new(line0.clone(), text.clone(), None));
            self.result = Some((format!("{line0} ="), text));
            self.tokens.clear();
            self.just_evaluated = true;
            self.cursor = None;
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
                Ok(r) => (
                    r.iter()
                        .enumerate()
                        .map(|(i, v)| {
                            format!(
                                "x{} = {}",
                                ["₁", "₂", "₃", "₄", "₅", "₆", "₇", "₈", "₉"]
                                    .get(i)
                                    .unwrap_or(&""),
                                display_num(*v)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(";  "),
                    None,
                ),
                Err(e) => (solve_error_text(&e).to_string(), None),
            };
            self.s
                .history
                .insert(0, HistItem::new(line.clone(), text.clone(), value));
            self.result = Some((line, text));
            self.tokens = value.map(|v| vec![Tok::Num(plain(v))]).unwrap_or_default();
            if let Some(v) = value {
                self.s.ans = v;
            }
        } else {
            match self.ev().eval(&self.tokens) {
                Ok(v) => {
                    let text = display_num(v);
                    self.s
                        .history
                        .insert(0, HistItem::new(line.clone(), text.clone(), Some(v)));
                    self.result = Some((format!("{line} ="), text));
                    self.tokens = vec![Tok::Num(plain(v))];
                    self.s.ans = v;
                }
                Err(e) => {
                    self.result = Some((format!("{line} ="), expr::error_text(&e).to_string()));
                    self.tokens.clear();
                }
            }
        }
        if self.s.history.len() > 500 {
            let mut n = 0;
            self.s.history.retain(|h| {
                n += 1;
                n <= 500 || h.pinned
            });
        }
        self.just_evaluated = true;
        self.cursor = None;
    }

    pub fn memory(&mut self, key: &str) {
        let cur = self.current_value();
        match key {
            "M1" | "M2" | "M3" | "M4" | "M5" => {
                self.s.mem_slot = (self.s.mem_slot + 1) % 5;
            }
            "MC" => self.s.mem[self.s.mem_slot] = 0.0,
            "M+" => {
                if let Some(v) = cur {
                    self.s.mem[self.s.mem_slot] += v;
                    self.just_evaluated = true;
                }
            }
            "M−" => {
                if let Some(v) = cur {
                    self.s.mem[self.s.mem_slot] -= v;
                    self.just_evaluated = true;
                }
            }
            "MR" => {
                let m = self.s.mem[self.s.mem_slot];
                if self.just_evaluated || self.tokens.is_empty() {
                    self.set_value(m);
                    self.just_evaluated = false;
                } else if !is_value_end(self.tokens.last()) {
                    self.tokens.push(Tok::Num(plain(m)));
                }
            }
            _ => {}
        }
    }

}
