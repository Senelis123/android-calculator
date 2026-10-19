use super::*;
impl App {
    pub(super) fn build_calc(&self, out: &mut Vec<W>, w: f32, h: f32, d: f32, t: &Theme) {
        let landscape = w > h;
        let pad = 12.0 * d;
        // top bar
        let bar_h = 40.0 * d;
        let chips: [(&str, Action, bool); 6] = [
            (
                crate::i18n::t("History"),
                Action::Nav(Screen::History),
                false,
            ),
            (
                crate::i18n::t("Scientific"),
                Action::ToggleSci,
                self.s.sci || landscape,
            ),
            (
                crate::i18n::t("Converter"),
                Action::Nav(Screen::Converter),
                false,
            ),
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
            out.push(W::Button {
                r,
                label: label.to_string(),
                px,
                kind: if *on { Kind::ChipOn } else { Kind::Chip },
                action: action.clone(),
            });
        }
        let top = pad + bar_h;
        let inner_w = w - 2.0 * pad;
        let avail = h - top - pad;
        let sci = self.s.sci || landscape;
        let mem_h = 38.0 * d;
        // display height
        let kf = [1.0, 0.8, 1.2][self.s.key_size as usize % 3];
        let disp_h = (if landscape {
            avail * 0.28
        } else if sci {
            avail * 0.22
        } else {
            avail * 0.26
        }) * kf;
        let disp = Rect::new(pad, top, inner_w, disp_h);
        self.build_display(out, disp, d, t);
        // memory row
        let mem = Rect::new(pad, disp.bottom(), inner_w, mem_h);
        let slot = format!("M{}", self.s.mem_slot + 1);
        let mkeys = [slot.as_str(), "MC", "M+", "M−", "MR"];
        let mw = mem.w / 5.0;
        for (i, k) in mkeys.iter().enumerate() {
            out.push(W::Button {
                r: Rect::new(mem.x + i as f32 * mw, mem.y, mw, mem.h),
                label: k.to_string(),
                px: 15.0 * d,
                kind: Kind::Flat,
                action: Action::Key(k.to_string()),
            });
        }
        let grid_top = mem.bottom() + 2.0 * d;
        let grid_h = h - pad - grid_top;
        let basic: [&str; 20] = [
            "C", "⌫", "÷", "×", "7", "8", "9", "−", "4", "5", "6", "+", "1", "2", "3", "=", "0",
            ",", "±", "%",
        ];
        let deg = if self.s.degrees { "DEG" } else { "RAD" };
        let scik: Vec<&str> = if self.sci_page >= 2 {
            crate::science::page_keys(self.sci_page as usize).to_vec()
        } else if self.sci_page == 0 {
            vec![
                "2nd", "(", ")", "◀", "▶", "x²", "xʸ", "√", "π", "e", "1/x", "sin", "cos", "tan",
                "n!", "ln", "log", "x", "Ans", deg,
            ]
        } else {
            vec![
                "2nd", "sin⁻¹", "cos⁻¹", "tan⁻¹", "mod", "sinh", "cosh", "tanh", "ⁿ√", "logₙ",
                "nCr", "nPr", ";", "EE", "a/b", "i", "<", ">", "↶", "↷",
            ]
        };
        let m = 3.0 * d;
        let place =
            |out: &mut Vec<W>, keys: &[&str], area: Rect, cols: usize, rows: usize, px: f32| {
                let (cw, rh) = (area.w / cols as f32, area.h / rows as f32);
                for (i, k) in keys.iter().enumerate() {
                    let (c, r) = ((i % cols) as f32, (i / cols) as f32);
                    let rect = Rect::new(
                        area.x + c * cw + m,
                        area.y + r * rh + m,
                        cw - 2.0 * m,
                        rh - 2.0 * m,
                    );
                    let kind = match *k {
                        "=" => Kind::Eq,
                        "÷" | "×" | "−" | "+" | "C" | "⌫" | "%" | "±" => Kind::Op,
                        k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) || k == "," => {
                            Kind::Digit
                        }
                        _ => Kind::Sci,
                    };
                    out.push(W::Button {
                        r: rect,
                        label: k.to_string(),
                        px,
                        kind,
                        action: Action::Key(k.to_string()),
                    });
                }
            };
        if landscape {
            let half = inner_w * 0.5;
            place(
                out,
                &scik,
                Rect::new(pad, grid_top, half, grid_h),
                4,
                5,
                15.0 * d,
            );
            place(
                out,
                &basic,
                Rect::new(pad + half, grid_top, half, grid_h),
                4,
                5,
                18.0 * d,
            );
        } else {
            // tablets: keep keys a sensible size; one-hand mode: narrower keypad on one side
            let mut kw = inner_w.min(560.0 * d);
            if self.s.one_hand > 0 {
                kw *= 0.8;
            }
            let kx = match self.s.one_hand {
                1 => pad + inner_w - kw,
                2 => pad,
                _ => pad + (inner_w - kw) / 2.0,
            };
            let (gt, gh) = if self.s.one_hand > 0 {
                let gh = grid_h * 0.85;
                (grid_top + grid_h - gh, gh)
            } else {
                (grid_top, grid_h)
            };
            if sci {
                let sci_h = gh * 5.0 / 10.0 * 0.8;
                let spx = if self.sci_page >= 2 { 13.5 * d } else { 16.0 * d };
                place(out, &scik, Rect::new(kx, gt, kw, sci_h), 4, 5, spx);
                place(
                    out,
                    &basic,
                    Rect::new(kx, gt + sci_h, kw, gh - sci_h),
                    4,
                    5,
                    22.0 * d,
                );
            } else {
                place(out, &basic, Rect::new(kx, gt, kw, gh), 4, 5, 22.0 * d);
            }
        }
        if self.popup {
            let pw = 240.0 * d;
            let r = Rect::new(disp.right() - pw, disp.y + 8.0 * d, pw, 44.0 * d);
            out.push(W::Fill {
                r: Rect {
                    y: r.y + 2.0 * d,
                    ..r
                },
                c: t.shadow,
                radius: 10.0 * d,
            });
            out.push(W::Fill {
                r,
                c: t.chip_on,
                radius: 10.0 * d,
            });
            out.push(W::Button {
                r: Rect { w: pw / 2.0, ..r },
                label: crate::i18n::t("Copy").into(),
                px: 15.0 * d,
                kind: Kind::ChipOn,
                action: Action::Copy,
            });
            out.push(W::Button {
                r: Rect {
                    x: r.x + pw / 2.0,
                    w: pw / 2.0,
                    ..r
                },
                label: crate::i18n::t("Paste").into(),
                px: 15.0 * d,
                kind: Kind::ChipOn,
                action: Action::Paste,
            });
        }
    }

    fn build_display(&self, out: &mut Vec<W>, r: Rect, d: f32, t: &Theme) {
        out.push(W::Fill {
            r: Rect::new(r.x, r.y + 2.0 * d, r.w, r.h - 4.0 * d),
            c: t.chip,
            radius: 16.0 * d,
        });
        let inner = r.inset(16.0 * d, 4.0 * d);
        // indicators
        let mut ind = vec![];
        if self.s.mem[self.s.mem_slot] != 0.0 {
            ind.push(format!("M{}", self.s.mem_slot + 1));
        }
        if self.sci_page > 0 {
            ind.push(format!("{}/{}", self.sci_page + 1, crate::science::SCI_PAGES));
        }
        if self.s.sci {
            ind.push(if self.s.degrees { "DEG" } else { "RAD" }.to_string());
        }
        if !ind.is_empty() {
            out.push(W::Text {
                r: Rect::new(inner.x, inner.y, inner.w, 18.0 * d),
                text: ind.join("  "),
                px: 12.0 * d,
                bold: true,
                c: t.dim,
                align: 0,
                wrap: false,
            });
        }
        let (small, big, big_bold) = match &self.result {
            Some((line, res)) => {
                let mut res = res.clone();
                if self.show_frac {
                    if let Some(f) = self
                        .s
                        .history
                        .first()
                        .and_then(|h| h.value)
                        .and_then(expr::fraction_text)
                    {
                        res = f;
                    }
                }
                (line.clone(), res, true)
            }
            None => {
                let mut e = match self.cursor {
                    Some(c) if c <= self.tokens.len() => format!(
                        "{}|{}",
                        display_tokens(&self.tokens[..c]),
                        display_tokens(&self.tokens[c..])
                    ),
                    _ => display_tokens(&self.tokens),
                };
                let open = self.open_parens();
                if open > 0 && !e.is_empty() {
                    e.push_str(&")".repeat(open as usize));
                }
                let e = if e.is_empty() { "0".to_string() } else { e };
                let has_op = self.tokens.iter().any(|t| !matches!(t, Tok::Num(_)));
                let prev = if has_op && !self.tokens.contains(&Tok::X) {
                    self.current_value()
                        .map(|v| format!("= {}", display_num(v)))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                (prev, e, !has_op)
            }
        };
        let (big_r, small_r) = if self.result.is_some() {
            (
                Rect::new(inner.x, inner.y + inner.h * 0.35, inner.w, inner.h * 0.65),
                Rect::new(inner.x, inner.y + 16.0 * d, inner.w, inner.h * 0.3),
            )
        } else {
            (
                Rect::new(
                    inner.x,
                    inner.y + 16.0 * d,
                    inner.w,
                    inner.h * 0.7 - 16.0 * d,
                ),
                Rect::new(inner.x, inner.y + inner.h * 0.7, inner.w, inner.h * 0.3),
            )
        };
        out.push(W::Text {
            r: small_r,
            text: small,
            px: 22.0 * d,
            bold: false,
            c: t.dim,
            align: 2,
            wrap: false,
        });
        if self.result.is_none() {
            let text = self.edit_text.clone().unwrap_or_else(|| expr::editable_text(&self.tokens));
            let shown = if text.is_empty() { "0".to_string() } else { text.clone() };
            let fonts = Fonts::new();
            let px = (44.0*d*font_scale()).min(big_r.w / fonts.width(&shown, 1.0, false).max(1.0));
            let tw = fonts.width(&shown, px, false);
            let mut cx = big_r.right() - tw;
            out.push(W::Text { r: big_r, text: shown.clone(), px:px/font_scale(), bold: false, c: t.text, align: 2, wrap: false });
            for (i,ch) in shown.chars().enumerate() {
                let cw = fonts.width(&ch.to_string(),px,false);
                out.push(W::Button { r: Rect::new(cx,big_r.y,cw/2.0,big_r.h), label:String::new(),px:0.0,kind:Kind::Invisible,action:Action::EditCursor(i) });
                out.push(W::Button { r: Rect::new(cx+cw/2.0,big_r.y,cw/2.0,big_r.h), label:String::new(),px:0.0,kind:Kind::Invisible,action:Action::EditCursor(i+1) });
                if self.edit_text.is_some() && self.edit_cursor == i { out.push(W::Fill {r:Rect::new(cx,big_r.y+big_r.h*0.25,2.0*d,big_r.h*0.5),c:t.key_eq,radius:0.0}); }
                cx += cw;
            }
            if self.edit_text.is_some() && self.edit_cursor == shown.chars().count() { out.push(W::Fill {r:Rect::new(cx-2.0*d,big_r.y+big_r.h*0.25,2.0*d,big_r.h*0.5),c:t.key_eq,radius:0.0}); }
        } else {
            out.push(W::Text { r: big_r, text: big, px:44.0*d,bold:big_bold,c:t.text,align:2,wrap:true });
            out.push(W::Button {r:small_r,label:String::new(),px:0.0,kind:Kind::Invisible,action:Action::EditCursor(0)});
        }
        if self.s.words && self.result.is_some() {
            if let Some(w) = self
                .s
                .history
                .first()
                .and_then(|h| h.value)
                .and_then(|v| expr::words(v, crate::i18n::lang()))
            {
                out.push(W::Text {
                    r: Rect::new(inner.x, inner.bottom() - 14.0 * d, inner.w, 14.0 * d),
                    text: w,
                    px: 12.0 * d,
                    bold: false,
                    c: t.dim,
                    align: 2,
                    wrap: false,
                });
            }
        }
    }

    pub(super) fn build_form_keys(&self, out: &mut Vec<W>, area: Rect, d: f32, t: &Theme) {
        out.push(W::Fill {
            r: area,
            c: t.bg,
            radius: 0.0,
        });
        out.push(W::Fill {
            r: Rect::new(0.0, area.y, area.w, 1.0 * d),
            c: t.line,
            radius: 0.0,
        });
        let def = &crate::tools::TOOLS[self.tool];
        let hex = self.tool == crate::tools::T_BASES && self.tool_mode == 3;
        let last: &'static str = if def.list { "add" } else { "next" };
        let mut keys: Vec<&'static str> = vec![
            "7",
            "8",
            "9",
            "⌫",
            "4",
            "5",
            "6",
            "C",
            "1",
            "2",
            "3",
            "±",
            "0",
            ",",
            if def.list { "undo" } else { "roll" },
            last,
        ];
        if self.tool != crate::tools::T_RANDOM && !def.list {
            keys[14] = "next";
            keys[15] = "done";
        }
        if hex {
            keys = vec![
                "7", "8", "9", "⌫", "4", "5", "6", "C", "1", "2", "3", "A", "0", "B", "C_", "D",
                "E", "F", "next", "done",
            ];
        }
        let cols = 4;
        let rows = keys.len().div_ceil(cols);
        let inner = area.inset(8.0 * d, 6.0 * d);
        let (cw, rh) = (inner.w / cols as f32, inner.h / rows as f32);
        for (i, k) in keys.iter().enumerate() {
            let r = Rect::new(
                inner.x + (i % cols) as f32 * cw + 3.0 * d,
                inner.y + (i / cols) as f32 * rh + 3.0 * d,
                cw - 6.0 * d,
                rh - 6.0 * d,
            );
            let (label, act): (String, &'static str) = match *k {
                "next" => (crate::i18n::t("Next").into(), "next"),
                "add" => (crate::i18n::t("Add").into(), "add"),
                "undo" => (crate::i18n::t("Remove last").into(), "undo"),
                "roll" => (crate::i18n::t("Again").into(), "roll"),
                "done" => (crate::i18n::t("Done").into(), "done"),
                "C_" => ("C".into(), "C_"),
                k => (k.to_string(), k),
            };
            let kind = match act {
                "add" | "done" | "roll" => Kind::Eq,
                "⌫" | "C" | "±" | "next" | "undo" => Kind::Op,
                _ => Kind::Digit,
            };
            let action = if act == "done" {
                Action::ClosePopup
            } else if act == "C_" {
                Action::FormKey("C_")
            } else {
                Action::FormKey(act)
            };
            out.push(W::Button {
                r,
                label,
                px: 19.0 * d,
                kind,
                action,
            });
        }
    }

    // Expression to graph: the calculator expression with x (an equation is graphed as left − right).
}
