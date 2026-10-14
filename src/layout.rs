use super::*;
impl App {
    /// Lays out the whole screen inside the system bars: everything is built in the free area and moved
    /// into place, while the background still covers the full window.
    pub fn build(&mut self, w: f32, h: f32, d: f32) -> Frame {
        let [l, t, r, b] = self.insets;
        if l + t + r + b < 1.0 {
            return self.build_inner(w, h, d);
        }
        let (iw, ih) = ((w - l - r).max(1.0), (h - t - b).max(1.0));
        let mut f = self.build_inner(iw, ih, d);
        for wd in f.widgets.iter_mut() {
            shift_xy(wd, l, t);
        }
        if let Some(a) = f.scroll_area.as_mut() {
            a.x += l;
            a.y += t;
        }
        if let Some(W::Fill { r, .. }) = f.widgets.first_mut() {
            *r = Rect::new(0.0, 0.0, w, h);
        }
        f
    }

    pub(super) fn build_inner(&mut self, w: f32, h: f32, d: f32) -> Frame {
        let mut out = vec![];
        let t = theme(self.dark());
        out.push(W::Fill {
            r: Rect::new(0.0, 0.0, w, h),
            c: t.bg,
            radius: 0.0,
        });
        let mut scroll_area = None;
        let mut page = (0, 0);
        match self.screen {
            Screen::Calc => self.build_calc(&mut out, w, h, d, &t),
            _ => {
                let body = self.build_header(&mut out, w, d, &t);
                let kp_h = if self.screen == Screen::Tool {
                    (h * 0.34).min(300.0 * d)
                } else {
                    0.0
                };
                let area = Rect::new(0.0, body, w, h - body - kp_h);
                if kp_h > 0.0 {
                    self.build_form_keys(&mut out, Rect::new(0.0, h - kp_h, w, kp_h), d, &t);
                }
                scroll_area = Some(area);
                let start = out.len();
                let content_h = self.build_page(&mut out, area, d, &t);
                self.content_h = content_h;
                let max = (content_h - area.h).max(0.0);
                self.scroll = self.scroll.clamp(0.0, max);
                for wd in &mut out[start..] {
                    shift(wd, -self.scroll);
                }
                page = (start, out.len());
            }
        }
        if let Some(b) = &self.banner {
            let r = Rect::new(12.0 * d, h - 60.0 * d, w - 24.0 * d, 48.0 * d);
            out.push(W::Fill {
                r,
                c: [0x22, 0x22, 0x22],
                radius: 10.0 * d,
            });
            out.push(W::Text {
                r: r.inset(12.0 * d, 0.0),
                text: b.clone(),
                px: 14.0 * d,
                bold: false,
                c: [0xFF, 0xFF, 0xFF],
                align: 1,
                wrap: false,
            });
            out.push(W::Button {
                r,
                label: String::new(),
                px: 0.0,
                kind: Kind::Invisible,
                action: Action::UpdateBanner,
            });
        } else if let Some(msg) = &self.toast {
            let tw = 40.0 * d + msg.chars().count() as f32 * 8.0 * d;
            let r = Rect::new((w - tw) / 2.0, h - 90.0 * d, tw, 36.0 * d);
            out.push(W::Fill {
                r,
                c: [0x33, 0x33, 0x33],
                radius: 18.0 * d,
            });
            out.push(W::Text {
                r,
                text: msg.clone(),
                px: 14.0 * d,
                bold: false,
                c: [0xFF, 0xFF, 0xFF],
                align: 1,
                wrap: false,
            });
        }
        if let Some(c) = &self.crash {
            let r = Rect::new(16.0 * d, h * 0.12, w - 32.0 * d, h * 0.72);
            out.push(W::Fill {
                r: Rect::new(0.0, 0.0, w, h),
                c: t.shadow,
                radius: 0.0,
            });
            out.push(W::Button {
                r: Rect::new(0.0, 0.0, w, h),
                label: String::new(),
                px: 0.0,
                kind: Kind::Invisible,
                action: Action::None,
            });
            out.push(W::Fill {
                r,
                c: t.bg,
                radius: 14.0 * d,
            });
            out.push(W::Text {
                r: Rect::new(r.x + 16.0 * d, r.y + 10.0 * d, r.w - 32.0 * d, 56.0 * d),
                text: crate::i18n::t("The app closed because of an error").into(),
                px: 17.0 * d,
                bold: true,
                c: t.text,
                align: 0,
                wrap: true,
            });
            out.push(W::Text {
                r: Rect::new(r.x + 16.0 * d, r.y + 66.0 * d, r.w - 32.0 * d, 40.0 * d),
                text: crate::i18n::t(
                    "Please copy or share this text and send it to the developer.",
                )
                .into(),
                px: 13.0 * d,
                bold: false,
                c: t.text,
                align: 0,
                wrap: true,
            });
            let mut shown: String = c.replace('\t', " ").chars().take(360).collect();
            if c.chars().count() > 360 {
                shown.push_str(" …");
            }
            out.push(W::Text {
                r: Rect::new(
                    r.x + 16.0 * d,
                    r.y + 112.0 * d,
                    r.w - 32.0 * d,
                    r.h - 190.0 * d,
                ),
                text: shown,
                px: 11.0 * d,
                bold: false,
                c: t.text,
                align: 0,
                wrap: true,
            });
            let bw = (r.w - 32.0 * d - 16.0 * d) / 3.0;
            let by = r.bottom() - 60.0 * d;
            out.push(W::Button {
                r: Rect::new(r.x + 16.0 * d, by, bw, 46.0 * d),
                label: crate::i18n::t("Copy").into(),
                px: 14.0 * d,
                kind: Kind::Chip,
                action: Action::Set("crash_copy"),
            });
            out.push(W::Button {
                r: Rect::new(r.x + 24.0 * d + bw, by, bw, 46.0 * d),
                label: crate::i18n::t("Share").into(),
                px: 14.0 * d,
                kind: Kind::Chip,
                action: Action::Set("crash_share"),
            });
            out.push(W::Button {
                r: Rect::new(r.x + 32.0 * d + 2.0 * bw, by, bw, 46.0 * d),
                label: crate::i18n::t("OK").into(),
                px: 14.0 * d,
                kind: Kind::ChipOn,
                action: Action::Set("crash_close"),
            });
        }
        Frame {
            widgets: out,
            scroll_area,
            page,
        }
    }

    pub fn graph_tokens(&self) -> Vec<Tok> {
        let src = if self.tokens.contains(&Tok::X) {
            self.tokens.clone()
        } else {
            self.s
                .history
                .iter()
                .find(|h| h.expr.contains('x'))
                .map(|_| vec![])
                .unwrap_or_default()
        };
        if let Some(p) = src
            .iter()
            .position(|t| matches!(t, Tok::Equals | Tok::Cmp(_)))
        {
            let mut v = vec![Tok::LParen];
            v.extend_from_slice(&src[..p]);
            v.push(Tok::RParen);
            v.push(Tok::Op('−'));
            v.push(Tok::LParen);
            v.extend_from_slice(&src[p + 1..]);
            v.push(Tok::RParen);
            v
        } else {
            src
        }
    }

    /// Returns the y where the page body starts.
    pub(super) fn build_header(&self, out: &mut Vec<W>, w: f32, d: f32, t: &Theme) -> f32 {
        let title = match self.screen {
            Screen::History => crate::i18n::t("History"),
            Screen::Vat => crate::i18n::t("VAT and tips"),
            Screen::Converter => crate::i18n::t("Converter"),
            Screen::PickUnit { .. } => crate::i18n::t("Choose a unit"),
            Screen::Settings => crate::i18n::t("Settings"),
            Screen::Photo => crate::i18n::t("Photo problem"),
            Screen::Gallery => crate::i18n::t("Choose a photo"),
            Screen::Calc => "",
            Screen::Tools => crate::i18n::t("Tools"),
            Screen::Tool => crate::i18n::t(crate::tools::TOOLS[self.tool].name),
            Screen::Markets => crate::i18n::t("Markets"),
            Screen::Graph => crate::i18n::t("Graph"),
            Screen::Ask => crate::i18n::t("Word problem"),
            Screen::WhatsNew => crate::i18n::t("What's new"),
            Screen::MarketDetail => "",
            Screen::Periodic => crate::i18n::t("Periodic table"),
            Screen::Element => "",
            Screen::Constants => crate::i18n::t("Constants"),
            Screen::Steps => crate::i18n::t("Equation steps"),
            Screen::Graph3D => crate::i18n::t("3D graph"),
            Screen::Quiz => "Quiz",
            Screen::Formulas => "My formulas",
            Screen::StatsCharts => "Statistics charts",
        };
        let title = match (&self.screen, &self.market_sel) {
            (Screen::MarketDetail, Some(q)) => format!("{}  {}", q.symbol, q.name),
            (Screen::Element, _) => {
                let e = &crate::science::ELEMENTS[self.elem_sel.min(crate::science::ELEMENTS.len() - 1)];
                format!("{}  {}", e.0, e.1)
            }
            _ => title.to_string(),
        };
        let h = 56.0 * d;
        out.push(W::Button {
            r: Rect::new(4.0 * d, 4.0 * d, 48.0 * d, 48.0 * d),
            label: "‹".into(),
            px: 30.0 * d,
            kind: Kind::Flat,
            action: Action::Back,
        });
        out.push(W::Text {
            r: Rect::new(56.0 * d, 0.0, w - 170.0 * d, h),
            text: title,
            px: 19.0 * d,
            bold: true,
            c: t.text,
            align: 0,
            wrap: false,
        });
        if self.screen == Screen::History && !self.s.history.is_empty() {
            out.push(W::Button {
                r: Rect::new(w - 110.0 * d, 10.0 * d, 98.0 * d, 36.0 * d),
                label: crate::i18n::t("Clear").into(),
                px: 14.0 * d,
                kind: Kind::Chip,
                action: Action::ClearHistory,
            });
        }
        out.push(W::Fill {
            r: Rect::new(0.0, h, w, 1.0 * d),
            c: t.line,
            radius: 0.0,
        });
        h + 1.0 * d
    }

    // Lays out the page body starting at area.y; returns content height.

}
