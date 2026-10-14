use super::*;
impl App {
    pub(super) fn build_page(&self, out: &mut Vec<W>, area: Rect, d: f32, t: &Theme) -> f32 {
        let pad = 16.0 * d;
        let x = area.x + pad;
        let w = area.w - 2.0 * pad;
        let mut y = area.y + 8.0 * d;
        let row = |out: &mut Vec<W>, y: &mut f32, label: &str, value: &str, action: Action| {
            let r = Rect::new(x, *y, w, 56.0 * d);
            out.push(W::Button {
                r,
                label: String::new(),
                px: 0.0,
                kind: Kind::Row,
                action,
            });
            out.push(W::Text {
                r: Rect::new(r.x + 12.0 * d, r.y, r.w * 0.55, r.h),
                text: label.into(),
                px: 15.0 * d,
                bold: false,
                c: t.dim,
                align: 0,
                wrap: false,
            });
            out.push(W::Text {
                r: Rect::new(r.x + r.w * 0.4, r.y, r.w * 0.6 - 12.0 * d, r.h),
                text: value.into(),
                px: 17.0 * d,
                bold: true,
                c: t.text,
                align: 2,
                wrap: false,
            });
            *y += 60.0 * d;
        };
        let heading = |out: &mut Vec<W>, y: &mut f32, text: &str| {
            *y += 10.0 * d;
            out.push(W::Text {
                r: Rect::new(x, *y, w, 28.0 * d),
                text: text.into(),
                px: 14.0 * d,
                bold: true,
                c: t.dim,
                align: 0,
                wrap: false,
            });
            *y += 32.0 * d;
        };
        let para = |out: &mut Vec<W>, y: &mut f32, text: &str, px: f32| {
            let lines = (text.chars().count() as f32 * px * 0.55 / w)
                .ceil()
                .max(1.0)
                + text.matches('\n').count() as f32;
            let hh = lines * px * 1.35 + 6.0 * d;
            out.push(W::Text {
                r: Rect::new(x, *y, w, hh),
                text: text.into(),
                px,
                bold: false,
                c: t.dim,
                align: 0,
                wrap: true,
            });
            *y += hh + 4.0 * d;
        };
        let chips = |out: &mut Vec<W>, y: &mut f32, items: &[(String, Action, bool)]| {
            let gap = 8.0 * d;
            let mut cx = x;
            for (label, action, on) in items {
                let cw = 28.0 * d + label.chars().count() as f32 * 8.5 * d;
                if cx + cw > x + w {
                    cx = x;
                    *y += 44.0 * d;
                }
                out.push(W::Button {
                    r: Rect::new(cx, *y, cw, 36.0 * d),
                    label: label.clone(),
                    px: 14.0 * d,
                    kind: if *on { Kind::ChipOn } else { Kind::Chip },
                    action: action.clone(),
                });
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
                out.push(W::Button {
                    r: sr,
                    label: String::new(),
                    px: 0.0,
                    kind: if searching { Kind::ChipOn } else { Kind::Row },
                    action: if searching {
                        Action::EndText
                    } else {
                        Action::StartText(0, 0)
                    },
                });
                let stext = if self.search.is_empty() && !searching {
                    format!("{}…", crate::i18n::t("Search"))
                } else {
                    format!("{}{}", self.search, if searching { "|" } else { "" })
                };
                out.push(W::Text {
                    r: sr.inset(12.0 * d, 0.0),
                    text: stext,
                    px: 15.0 * d,
                    bold: false,
                    c: if searching { t.chip_on_text } else { t.dim },
                    align: 0,
                    wrap: false,
                });
                y += 56.0 * d;
                chips(
                    out,
                    &mut y,
                    &[
                        (
                            crate::i18n::t("Export").to_string(),
                            Action::Set("export"),
                            false,
                        ),
                        (
                            crate::i18n::t("Backup").to_string(),
                            Action::Set("backup"),
                            false,
                        ),
                        (
                            crate::i18n::t("Restore").to_string(),
                            Action::Set("restore"),
                            false,
                        ),
                    ],
                );
                if self.s.history.is_empty() {
                    para(
                        out,
                        &mut y,
                        crate::i18n::t("History is empty. Your calculations will appear here."),
                        15.0 * d,
                    );
                }
                let q = self.search.to_lowercase();
                let mut idx: Vec<usize> = (0..self.s.history.len())
                    .filter(|&i| {
                        let h = &self.s.history[i];
                        q.is_empty()
                            || h.expr.to_lowercase().contains(&q)
                            || h.result.to_lowercase().contains(&q)
                            || h.note.to_lowercase().contains(&q)
                            || h.expr
                                .replace(|c: char| !c.is_ascii_digit(), "")
                                .contains(&q)
                    })
                    .collect();
                idx.sort_by_key(|&i| !self.s.history[i].pinned);
                for i in idx {
                    let hi = &self.s.history[i];
                    let editing = self.text_target == Some((1, i));
                    let has_note = !hi.note.is_empty() || editing;
                    let rh = if has_note { 88.0 } else { 64.0 } * d;
                    let r = Rect::new(x, y, w, rh);
                    out.push(W::Button {
                        r,
                        label: String::new(),
                        px: 0.0,
                        kind: Kind::Row,
                        action: Action::LoadHistory(i),
                    });
                    let tag = if hi.photo { "◉ " } else { "" };
                    out.push(W::Text {
                        r: Rect::new(r.x + 144.0 * d, r.y + 4.0 * d, r.w - 156.0 * d, 26.0 * d),
                        text: format!("{tag}{}", hi.expr),
                        px: 14.0 * d,
                        bold: false,
                        c: t.dim,
                        align: 2,
                        wrap: false,
                    });
                    out.push(W::Text {
                        r: Rect::new(r.x + 144.0 * d, r.y + 28.0 * d, r.w - 156.0 * d, 32.0 * d),
                        text: hi.result.clone(),
                        px: 20.0 * d,
                        bold: true,
                        c: t.text,
                        align: 2,
                        wrap: false,
                    });
                    if has_note {
                        out.push(W::Text {
                            r: Rect::new(r.x + 12.0 * d, r.y + 60.0 * d, r.w - 24.0 * d, 24.0 * d),
                            text: format!("✎ {}{}", hi.note, if editing { "|" } else { "" }),
                            px: 13.0 * d,
                            bold: false,
                            c: t.dim,
                            align: 0,
                            wrap: false,
                        });
                    }
                    out.push(W::Button {
                        r: Rect::new(r.x + 4.0 * d, r.y + 8.0 * d, 42.0 * d, 44.0 * d),
                        label: if hi.pinned {
                            "★".into()
                        } else {
                            "☆".into()
                        },
                        px: 20.0 * d,
                        kind: Kind::Flat,
                        action: Action::Pin(i),
                    });
                    out.push(W::Button {
                        r: Rect::new(r.x + 48.0 * d, r.y + 8.0 * d, 42.0 * d, 44.0 * d),
                        label: "✎".into(),
                        px: 18.0 * d,
                        kind: Kind::Flat,
                        action: if editing {
                            Action::EndText
                        } else {
                            Action::StartText(1, i)
                        },
                    });
                    out.push(W::Button {r:Rect::new(r.x+96.0*d,r.y+8.0*d,42.0*d,44.0*d),label:"↗".into(),px:17.0*d,kind:Kind::Flat,action:Action::ShareHistoryImage(i)});
                    y += rh + 4.0 * d;
                }
                if !self.s.history.is_empty() {
                    para(
                        out,
                        &mut y,
                        crate::i18n::t(
                            "Swipe an entry sideways to delete it. ★ keeps it at the top.",
                        ),
                        12.0 * d,
                    );
                }
            }
            Screen::Vat => {
                let v = value.unwrap_or(0.0);
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Amount (from calculator)"),
                    &display_num(v),
                    Action::None,
                );
                heading(out, &mut y, "PVM 21 %");
                row(
                    out,
                    &mut y,
                    "Su PVM (+21 %)",
                    &display_num(v * 1.21),
                    Action::UseValue(v * 1.21),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Without VAT (amount incl. VAT)"),
                    &display_num(v / 1.21),
                    Action::UseValue(v / 1.21),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("VAT from amount"),
                    &display_num(v * 0.21),
                    Action::UseValue(v * 0.21),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("VAT inside amount incl. VAT"),
                    &display_num(v - v / 1.21),
                    Action::UseValue(v - v / 1.21),
                );
                heading(out, &mut y, crate::i18n::t("Tip"));
                let tips = [5.0, 10.0, 15.0, 20.0];
                chips(
                    out,
                    &mut y,
                    &tips
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (format!("{} %", *p as i32), Action::Tip(i), self.s.tip == i))
                        .collect::<Vec<_>>(),
                );
                let p = tips[self.s.tip.min(3)] / 100.0;
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Tip"),
                    &display_num(v * p),
                    Action::UseValue(v * p),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Total"),
                    &display_num(v * (1.0 + p)),
                    Action::UseValue(v * (1.0 + p)),
                );
                heading(out, &mut y, crate::i18n::t("Split between people"));
                chips(
                    out,
                    &mut y,
                    &(1..=6)
                        .map(|n| (n.to_string(), Action::People(n), self.s.people == n))
                        .collect::<Vec<_>>(),
                );
                let each = v * (1.0 + p) / self.s.people.max(1) as f64;
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Per person"),
                    &display_num(each),
                    Action::UseValue(each),
                );
                para(
                    out,
                    &mut y,
                    crate::i18n::t("Tap a row to move the result to the calculator."),
                    13.0 * d,
                );
            }
            Screen::Converter => {
                let c = self.s.conv_cat;
                let mut order: Vec<usize> = self
                    .s
                    .fav_cats
                    .iter()
                    .cloned()
                    .filter(|i| *i < CATEGORIES.len())
                    .collect();
                for i in 0..CATEGORIES.len() {
                    if !order.contains(&i) {
                        order.push(i);
                    }
                }
                chips(
                    out,
                    &mut y,
                    &order
                        .iter()
                        .map(|&i| {
                            (
                                format!(
                                    "{}{}",
                                    if self.s.fav_cats.contains(&i) {
                                        "★ "
                                    } else {
                                        ""
                                    },
                                    crate::i18n::t(CATEGORIES[i].name)
                                ),
                                Action::ConvCat(i),
                                i == c,
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                let fav = self.s.fav_cats.contains(&c);
                out.push(W::Button {
                    r: Rect::new(x, y - 6.0 * d, 200.0 * d, 34.0 * d),
                    label: if fav {
                        format!("★ {}", crate::i18n::t("Favorite"))
                    } else {
                        format!("☆ {}", crate::i18n::t("Add to favorites"))
                    },
                    px: 13.0 * d,
                    kind: Kind::Flat,
                    action: Action::Set("fav"),
                });
                y += 36.0 * d;
                let names = convert::unit_names(c, &self.s.rates);
                let (fi, ti) = (
                    self.s.conv_from[c].min(names.len().saturating_sub(1)),
                    self.s.conv_to[c].min(names.len().saturating_sub(1)),
                );
                let v = value.unwrap_or(1.0);
                row(
                    out,
                    &mut y,
                    &crate::i18n::tf(
                        "From: {}  ▾",
                        &[&names.get(fi).cloned().unwrap_or_default()],
                    ),
                    &display_num(v),
                    Action::Nav(Screen::PickUnit { from: true }),
                );
                out.push(W::Button {
                    r: Rect::new(x + w / 2.0 - 30.0 * d, y - 6.0 * d, 60.0 * d, 36.0 * d),
                    label: "⇅".into(),
                    px: 20.0 * d,
                    kind: Kind::Chip,
                    action: Action::SwapUnits,
                });
                y += 34.0 * d;
                let res = convert::convert(c, fi, ti, v, &self.s.rates);
                row(
                    out,
                    &mut y,
                    &crate::i18n::tf("To: {}  ▾", &[&names.get(ti).cloned().unwrap_or_default()]),
                    &res.map(display_num).unwrap_or("—".into()),
                    Action::Nav(Screen::PickUnit { from: false }),
                );
                if let Some(r) = res {
                    out.push(W::Button {
                        r: Rect::new(x, y, w, 40.0 * d),
                        label: crate::i18n::t("Move result to calculator").into(),
                        px: 14.0 * d,
                        kind: Kind::Chip,
                        action: Action::UseValue(r),
                    });
                    y += 52.0 * d;
                }
                if c == CURRENCY_CAT {
                    let note = match &self.s.rates {
                        Some(r) => crate::i18n::tf(
                            "European Central Bank rates, {}. {}",
                            &[&r.date, &self.rates_status],
                        ),
                        None => crate::i18n::tf(
                            "Rates not downloaded yet. Internet needed. {}",
                            &[&self.rates_status],
                        ),
                    };
                    para(out, &mut y, &note, 13.0 * d);
                    out.push(W::Button {
                        r: Rect::new(x, y, 160.0 * d, 36.0 * d),
                        label: crate::i18n::t("Update rates").into(),
                        px: 14.0 * d,
                        kind: Kind::Chip,
                        action: Action::RefreshRates,
                    });
                    out.push(W::Button {
                        r: Rect::new(x + 170.0 * d, y, 190.0 * d, 36.0 * d),
                        label: crate::i18n::t("Rate history (90 days)").into(),
                        px: 14.0 * d,
                        kind: Kind::Chip,
                        action: Action::Set("ratehist"),
                    });
                    y += 48.0 * d;
                    let (fname, tname) = (
                        names.get(fi).cloned().unwrap_or_default(),
                        names.get(ti).cloned().unwrap_or_default(),
                    );
                    if let Some((a, b, pts)) = &self.rate_hist {
                        if *a == fname && *b == tname && pts.len() > 1 {
                            let r = Rect::new(x, y, w, w * 0.55);
                            out.push(W::Fill {
                                r,
                                c: t.chip,
                                radius: 8.0 * d,
                            });
                            let lo = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
                            let hi = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                            let span = if hi > lo { hi - lo } else { 1.0 };
                            let line: Vec<(f32, f32)> = pts
                                .iter()
                                .enumerate()
                                .map(|(i, p)| {
                                    (
                                        r.x + 6.0 * d
                                            + (r.w - 12.0 * d) * i as f32 / (pts.len() - 1) as f32,
                                        r.y + 24.0 * d
                                            + (r.h - 48.0 * d) * (1.0 - ((p.1 - lo) / span) as f32),
                                    )
                                })
                                .collect();
                            out.push(W::Plot {
                                r,
                                lines: vec![(1, line)],
                            });
                            out.push(W::Text {
                                r: Rect::new(
                                    r.x + 6.0 * d,
                                    r.y + 2.0 * d,
                                    r.w - 12.0 * d,
                                    20.0 * d,
                                ),
                                text: format!(
                                    "1 {a} = {} {b}  (max)",
                                    display_num((hi * 1e4).round() / 1e4)
                                ),
                                px: 11.0 * d,
                                bold: false,
                                c: t.dim,
                                align: 0,
                                wrap: false,
                            });
                            out.push(W::Text {
                                r: Rect::new(
                                    r.x + 6.0 * d,
                                    r.bottom() - 22.0 * d,
                                    r.w - 12.0 * d,
                                    20.0 * d,
                                ),
                                text: format!(
                                    "{} … {}   min {}",
                                    pts[0].0,
                                    pts[pts.len() - 1].0,
                                    display_num((lo * 1e4).round() / 1e4)
                                ),
                                px: 11.0 * d,
                                bold: false,
                                c: t.dim,
                                align: 0,
                                wrap: false,
                            });
                            y += r.h + 10.0 * d;
                        }
                    }
                }
                para(
                    out,
                    &mut y,
                    crate::i18n::t("Value is taken from the calculator display."),
                    13.0 * d,
                );
            }
            Screen::PickUnit { from } => {
                let c = self.s.conv_cat;
                let sel = if from {
                    self.s.conv_from[c]
                } else {
                    self.s.conv_to[c]
                };
                for (i, n) in convert::unit_names(c, &self.s.rates).iter().enumerate() {
                    row(
                        out,
                        &mut y,
                        n,
                        if i == sel { "✓" } else { "" },
                        Action::PickUnit(i),
                    );
                }
            }
            Screen::Settings => {
                let ln = if self.s.lang < 0 {
                    crate::i18n::t("Phone language")
                } else {
                    crate::i18n::LANG_NAMES[self.s.lang as usize]
                };
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Language"),
                    ln,
                    Action::CycleLang,
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Theme"),
                    [
                        crate::i18n::t("Automatic"),
                        crate::i18n::t("Light"),
                        crate::i18n::t("Dark"),
                    ][self.s.theme as usize % 3],
                    Action::CycleTheme,
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Vibrate on tap"),
                    if self.s.vibrate {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::ToggleVibrate,
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Angle unit"),
                    if self.s.degrees {
                        crate::i18n::t("Degrees")
                    } else {
                        crate::i18n::t("Radians")
                    },
                    Action::ToggleAngle,
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Scientific mode"),
                    if self.s.sci {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::ToggleSci,
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Decimal sign"),
                    if self.s.decimal == 1 { "." } else { "," },
                    Action::Set("decimal"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Digit grouping"),
                    [
                        crate::i18n::t("Space"),
                        crate::i18n::t("None"),
                        if self.s.decimal == 1 { "," } else { "." },
                        "'",
                    ][self.s.grouping as usize % 4],
                    Action::Set("grouping"),
                );
                let fixed = if self.s.fixed < 0 {
                    crate::i18n::t("Automatic").to_string()
                } else {
                    self.s.fixed.to_string()
                };
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Decimal places"),
                    &fixed,
                    Action::Set("fixed"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Result in words"),
                    if self.s.words {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("words"),
                );
                row(
                    out,
                    &mut y,
                    "Show exact fraction result",
                    if self.show_frac { "On" } else { "Off" },
                    Action::Set("fraction"),
                );
                heading(out, &mut y, crate::i18n::t("Look"));
                let acc = [
                    crate::i18n::t("Classic"),
                    crate::i18n::t("Blue"),
                    crate::i18n::t("Green"),
                    crate::i18n::t("Orange"),
                    crate::i18n::t("Purple"),
                    crate::i18n::t("Red"),
                    crate::i18n::t("Phone colors"),
                ][self.s.accent as usize % 7];
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Color"),
                    acc,
                    Action::Set("accent"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("High contrast"),
                    if self.s.contrast {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("contrast"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Button size"),
                    [
                        crate::i18n::t("Normal"),
                        crate::i18n::t("Large"),
                        crate::i18n::t("Small"),
                    ][self.s.key_size as usize % 3],
                    Action::Set("keys"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Text size"),
                    [
                        crate::i18n::t("Normal"),
                        crate::i18n::t("Large"),
                        crate::i18n::t("Extra large"),
                        crate::i18n::t("Small"),
                    ][self.s.font_size as usize % 4],
                    Action::Set("font"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("One-hand mode"),
                    [
                        crate::i18n::t("Off"),
                        crate::i18n::t("Right hand"),
                        crate::i18n::t("Left hand"),
                    ][self.s.one_hand as usize % 3],
                    Action::Set("onehand"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Press animation"),
                    if self.s.animations {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("anim"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("App icon"),
                    [
                        crate::i18n::t("Classic"),
                        crate::i18n::t("Blue"),
                        crate::i18n::t("Dark"),
                    ][self.s.icon as usize % 3],
                    Action::Set("icon"),
                );
                heading(out, &mut y, crate::i18n::t("Behavior"));
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Sound on tap"),
                    if self.s.sound {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("sound"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Vibration strength"),
                    &format!("{} ms", self.s.vib_ms),
                    Action::Set("vib"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Read results aloud"),
                    if self.s.speak {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("speak"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Start screen"),
                    [
                        crate::i18n::t("Calculator"),
                        crate::i18n::t("Converter"),
                        crate::i18n::t("Tools"),
                        crate::i18n::t("History"),
                    ][self.s.start_screen as usize % 4],
                    Action::Set("start"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Floating calculator"),
                    "›",
                    Action::Set("floating"),
                );
                heading(out, &mut y, crate::i18n::t("Updates"));
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Check for updates in the background"),
                    if self.s.bg_check {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("bgcheck"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Beta versions"),
                    if self.s.beta {
                        crate::i18n::t("On")
                    } else {
                        crate::i18n::t("Off")
                    },
                    Action::Set("beta"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Check for updates"),
                    crate::i18n::t("Check"),
                    Action::CheckUpdate,
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("What's new"),
                    "›",
                    Action::Set("whatsnew"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Report a problem"),
                    "›",
                    Action::Set("bug"),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Version"),
                    version(),
                    Action::None,
                );
                para(
                    out,
                    &mut y,
                    &crate::i18n::tf(
                        "Updates: github.com/{}\nCalculator written in Rust.",
                        &[&crate::update::UPDATE_REPO],
                    ),
                    13.0 * d,
                );
            }
            Screen::Tools => {
                row(out,&mut y,"Quiz","›",Action::Nav(Screen::Quiz));
                row(out,&mut y,"My formulas","›",Action::Nav(Screen::Formulas));
                row(out,&mut y,"Statistics charts","›",Action::Nav(Screen::StatsCharts));
                row(out,&mut y,"Share calculation as image","›",Action::Set("share_image"));
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Word problem"),
                    "›",
                    Action::Nav(Screen::Ask),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("VAT and tips"),
                    "›",
                    Action::Nav(Screen::Vat),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Graph"),
                    "›",
                    Action::Nav(Screen::Graph),
                );
                row(
                    out,
                    &mut y,
                    crate::i18n::t("Stocks & cryptocurrencies"),
                    "›",
                    Action::Nav(Screen::Markets),
                );
                row(out, &mut y, crate::i18n::t("Equation steps"), "›", Action::Nav(Screen::Steps));
                row(out, &mut y, crate::i18n::t("3D graph"), "›", Action::Nav(Screen::Graph3D));
                row(out, &mut y, crate::i18n::t("Periodic table"), "›", Action::Nav(Screen::Periodic));
                row(out, &mut y, crate::i18n::t("Constants"), "›", Action::Nav(Screen::Constants));
                for (i, td) in crate::tools::TOOLS.iter().enumerate() {
                    row(
                        out,
                        &mut y,
                        crate::i18n::t(td.name),
                        "›",
                        Action::OpenTool(i),
                    );
                }
            }
            Screen::Tool => {
                let tool = self.tool;
                let def = &crate::tools::TOOLS[tool];
                if !def.modes.is_empty() {
                    let items: Vec<(String, Action, bool)> = def
                        .modes
                        .iter()
                        .enumerate()
                        .map(|(i, m)| {
                            (
                                crate::i18n::t(m).to_string(),
                                Action::ToolMode(i),
                                i == self.tool_mode,
                            )
                        })
                        .collect();
                    chips(out, &mut y, &items);
                }
                let vis = crate::tools::visible_fields(tool, self.tool_mode);
                let grid = tool == crate::tools::T_SYSTEMS || tool == crate::tools::T_MATRIX;
                let cols = if tool == crate::tools::T_SYSTEMS {
                    if self.tool_mode == 0 {
                        3
                    } else {
                        4
                    }
                } else if tool == crate::tools::T_MATRIX {
                    if self.tool_mode == 0 {
                        2
                    } else {
                        3
                    }
                } else {
                    1
                };
                if tool == crate::tools::T_SYSTEMS {
                    para(
                        out,
                        &mut y,
                        if self.tool_mode == 0 {
                            "a1·x + b1·y = c1\na2·x + b2·y = c2"
                        } else {
                            "a·x + b·y + c·z = d"
                        },
                        14.0 * d,
                    );
                }
                let cw = w / cols as f32;
                for (k, f) in vis.iter().enumerate() {
                    let (cx, cy) = if grid {
                        (x + (k % cols) as f32 * cw, y + (k / cols) as f32 * 64.0 * d)
                    } else {
                        (x, y + k as f32 * 64.0 * d)
                    };
                    let r = Rect::new(
                        cx + if grid { 3.0 * d } else { 0.0 },
                        cy,
                        if grid { cw - 6.0 * d } else { w },
                        58.0 * d,
                    );
                    let focused = *f == self.focus;
                    out.push(W::Button {
                        r,
                        label: String::new(),
                        px: 0.0,
                        kind: if focused { Kind::ChipOn } else { Kind::Row },
                        action: Action::Focus(*f),
                    });
                    let (lc, vc) = if focused {
                        (t.chip_on_text, t.chip_on_text)
                    } else {
                        (t.dim, t.text)
                    };
                    let label = crate::i18n::t(crate::tools::field_label(tool, self.tool_mode, *f));
                    let raw = &self.fields[*f];
                    let shown = if tool == crate::tools::T_BASES {
                        raw.clone()
                    } else if raw.is_empty() {
                        String::new()
                    } else {
                        expr::display_raw(&raw.replacen('-', "−", 1).replace('−', "-"))
                    };
                    if grid {
                        out.push(W::Text {
                            r: Rect::new(r.x + 8.0 * d, r.y + 2.0 * d, r.w - 16.0 * d, 20.0 * d),
                            text: label.into(),
                            px: 12.0 * d,
                            bold: false,
                            c: lc,
                            align: 0,
                            wrap: false,
                        });
                        out.push(W::Text {
                            r: Rect::new(r.x + 8.0 * d, r.y + 22.0 * d, r.w - 16.0 * d, 32.0 * d),
                            text: shown,
                            px: 17.0 * d,
                            bold: true,
                            c: vc,
                            align: 2,
                            wrap: false,
                        });
                    } else {
                        out.push(W::Text {
                            r: Rect::new(r.x + 12.0 * d, r.y, r.w * 0.6, r.h),
                            text: label.into(),
                            px: 15.0 * d,
                            bold: false,
                            c: lc,
                            align: 0,
                            wrap: false,
                        });
                        out.push(W::Text {
                            r: Rect::new(r.x + r.w * 0.4, r.y, r.w * 0.6 - 12.0 * d, r.h),
                            text: if focused { format!("{shown}|") } else { shown },
                            px: 18.0 * d,
                            bold: true,
                            c: vc,
                            align: 2,
                            wrap: false,
                        });
                    }
                }
                let n_rows = if grid {
                    vis.len().div_ceil(cols)
                } else {
                    vis.len()
                };
                y += n_rows as f32 * 64.0 * d + 6.0 * d;
                if def.list && !self.list.is_empty() {
                    para(
                        out,
                        &mut y,
                        &self
                            .list
                            .iter()
                            .map(|v| display_num(*v))
                            .collect::<Vec<_>>()
                            .join(";  "),
                        14.0 * d,
                    );
                }
                let outs = if tool == crate::tools::T_BASES {
                    crate::tools::bases(&self.fields[0], self.tool_mode)
                } else {
                    crate::tools::compute(
                        tool,
                        self.tool_mode,
                        &self.field_values(),
                        &self.list,
                        self.seed,
                    )
                };
                heading(out, &mut y, crate::i18n::t("Result"));
                for (label, val) in outs {
                    if label.is_empty() {
                        para(out, &mut y, &val, 13.0 * d);
                        continue;
                    }
                    if val.contains('\n') {
                        heading(out, &mut y, &label);
                        for line in val.lines() {
                            out.push(W::Text {
                                r: Rect::new(x, y, w, 30.0 * d),
                                text: line.into(),
                                px: 18.0 * d,
                                bold: true,
                                c: t.text,
                                align: 0,
                                wrap: false,
                            });
                            y += 32.0 * d;
                        }
                        continue;
                    }
                    let num = val
                        .trim_end_matches(" €")
                        .trim_end_matches(" %")
                        .trim_end_matches(" l")
                        .replace(GROUPSP, "")
                        .replace('−', "-")
                        .replace(expr::dec_char(), ".");
                    let act = match num.parse::<f64>() {
                        Ok(v) => Action::UseValue(v),
                        Err(_) => Action::None,
                    };
                    row(out, &mut y, &label, &val, act);
                }
                para(
                    out,
                    &mut y,
                    crate::i18n::t("Tap a result to use it in the calculator."),
                    12.0 * d,
                );
            }
            Screen::Ask => {
                let typing = self.text_target.map(|t| t.0 == 2).unwrap_or(false);
                para(out, &mut y, crate::i18n::t("Say or type a problem in words, e.g. \"15 percent of 240\" or \"a book costs 12 euros, how much do 3 cost\"."), 14.0 * d);
                let tr = Rect::new(x, y, w, 96.0 * d);
                out.push(W::Button {
                    r: tr,
                    label: String::new(),
                    px: 0.0,
                    kind: if typing { Kind::ChipOn } else { Kind::Row },
                    action: if typing {
                        Action::EndText
                    } else {
                        Action::StartText(2, 0)
                    },
                });
                let tt = if self.ask_text.is_empty() && !typing {
                    format!("{}…", crate::i18n::t("Tap to type"))
                } else {
                    format!("{}{}", self.ask_text, if typing { "|" } else { "" })
                };
                out.push(W::Text {
                    r: tr.inset(12.0 * d, 8.0 * d),
                    text: tt,
                    px: 16.0 * d,
                    bold: false,
                    c: if typing { t.chip_on_text } else { t.text },
                    align: 0,
                    wrap: true,
                });
                y += 104.0 * d;
                chips(
                    out,
                    &mut y,
                    &[
                        (
                            crate::i18n::t("Speak").to_string(),
                            Action::Set("ask_voice"),
                            false,
                        ),
                        (
                            crate::i18n::t("Paste").to_string(),
                            Action::Set("ask_paste"),
                            false,
                        ),
                        (
                            crate::i18n::t("Clear").to_string(),
                            Action::Set("ask_clear"),
                            false,
                        ),
                    ],
                );
                let sr = Rect::new(x, y, w, 52.0 * d);
                out.push(W::Button {
                    r: sr,
                    label: crate::i18n::t("Solve").into(),
                    px: 17.0 * d,
                    kind: Kind::Eq,
                    action: Action::Set("ask_solve"),
                });
                y += 62.0 * d;
                if !self.ask_status.is_empty() {
                    para(out, &mut y, &self.ask_status.clone(), 14.0 * d);
                }
                if let Some((e, a, nano)) = self.ask_out.clone() {
                    heading(out, &mut y, crate::i18n::t("Result"));
                    let rr = Rect::new(x, y, w, 40.0 * d);
                    out.push(W::Text {
                        r: rr,
                        text: format!("{e} ="),
                        px: 18.0 * d,
                        bold: false,
                        c: t.dim,
                        align: 0,
                        wrap: true,
                    });
                    y += 42.0 * d;
                    out.push(W::Text {
                        r: Rect::new(x, y, w, 48.0 * d),
                        text: a,
                        px: 30.0 * d,
                        bold: true,
                        c: t.text,
                        align: 0,
                        wrap: false,
                    });
                    y += 54.0 * d;
                    para(
                        out,
                        &mut y,
                        if nano {
                            crate::i18n::t("Understood by Gemini Nano on this phone, calculated by the app. Check that the expression matches the problem.")
                        } else {
                            crate::i18n::t("Understood offline by the app. Check that the expression matches the problem.")
                        },
                        13.0 * d,
                    );
                    row(
                        out,
                        &mut y,
                        crate::i18n::t("Open in calculator"),
                        "›",
                        Action::Set("ask_open"),
                    );
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
            Screen::Markets => {
                chips(
                    out,
                    &mut y,
                    &[
                        (
                            crate::i18n::t("Crypto").into(),
                            Action::MarketKind(MarketKind::Crypto),
                            self.market_kind == MarketKind::Crypto,
                        ),
                        (
                            crate::i18n::t("Stocks").into(),
                            Action::MarketKind(MarketKind::Stocks),
                            self.market_kind == MarketKind::Stocks,
                        ),
                        (
                            crate::i18n::t("Refresh").into(),
                            Action::MarketRefresh,
                            false,
                        ),
                    ],
                );
                para(out, &mut y, crate::i18n::t("Informational market quotes. Tap a line to see its price history. Prices can be delayed and data providers may change."), 12.0 * d);
                if !self.market_status.is_empty() {
                    para(out, &mut y, &self.market_status, 12.0 * d);
                }
                if self.market_quotes.is_empty() {
                    para(
                        out,
                        &mut y,
                        crate::i18n::t(
                            "No quote data yet. Tap Refresh and make sure the device is online.",
                        ),
                        15.0 * d,
                    );
                } else {
                    for (qi, q) in self.market_quotes.iter().enumerate() {
                        let change = q
                            .change_pct
                            .map(|v| format!("{:+.2}%", v))
                            .unwrap_or_else(|| "—".into());
                        let value = format!("{}  {}", fmt_price(q.price), change);
                        row(
                            out,
                            &mut y,
                            &format!("{}  {}", q.symbol, q.name),
                            &value,
                            Action::MarketOpen(qi),
                        );
                        para(
                            out,
                            &mut y,
                            &crate::i18n::tf("{} • 24h change", &[&q.currency]),
                            11.0 * d,
                        );
                    }
                }
            }
            Screen::WhatsNew => {
                for line in crate::i18n::t(WHATS_NEW).split('\n') {
                    let px = 14.5 * d;
                    let lines = (line.chars().count() as f32 * px * 0.6 / (w - 4.0 * d))
                        .ceil()
                        .max(1.0);
                    let hh = lines * px * 1.4 + 8.0 * d;
                    out.push(W::Text {
                        r: Rect::new(x, y, w, hh),
                        text: line.into(),
                        px,
                        bold: false,
                        c: t.text,
                        align: 0,
                        wrap: true,
                    });
                    y += hh + 4.0 * d;
                }
                y += 8.0 * d;
                out.push(W::Button {
                    r: Rect::new(x, y, w, 52.0 * d),
                    label: crate::i18n::t("OK").into(),
                    px: 17.0 * d,
                    kind: Kind::ChipOn,
                    action: Action::Set("closenew"),
                });
                y += 64.0 * d;
            }
            _ => {return self.build_advanced_page(out,area,d,t);}
        }
        y - area.y + 24.0 * d
    }
}
