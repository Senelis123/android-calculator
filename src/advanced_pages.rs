use super::*;
impl App {
    pub(super) fn build_advanced_page(&self, out: &mut Vec<W>, area: Rect, d: f32, t: &Theme) -> f32 {
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
        match self.screen {
            Screen::MarketDetail => {
                if let Some(q) = &self.market_sel {
                    let change = q
                        .change_pct
                        .map(|v| format!("{:+.2}%", v))
                        .unwrap_or_else(|| "—".into());
                    out.push(W::Text {
                        r: Rect::new(x, y, w, 44.0 * d),
                        text: format!("{} {}", fmt_price(q.price), q.currency),
                        px: 30.0 * d,
                        bold: true,
                        c: t.text,
                        align: 0,
                        wrap: false,
                    });
                    y += 46.0 * d;
                    out.push(W::Text {
                        r: Rect::new(x, y, w, 24.0 * d),
                        text: crate::i18n::tf("{} • 24h change", &[&change]),
                        px: 14.0 * d,
                        bold: false,
                        c: if q.change_pct.unwrap_or(0.0) >= 0.0 {
                            [0x2E, 0x9E, 0x5B]
                        } else {
                            [0xD9, 0x4A, 0x4A]
                        },
                        align: 0,
                        wrap: false,
                    });
                    y += 34.0 * d;
                    let items: Vec<(String, Action, bool)> = crate::market::RANGES
                        .iter()
                        .enumerate()
                        .map(|(i, r)| (r.0.to_string(), Action::MarketRange(i), i == self.market_range))
                        .collect();
                    chips(out, &mut y, &items);
                    let gh = w * 0.75;
                    let r = Rect::new(x, y, w, gh);
                    out.push(W::Fill {
                        r,
                        c: t.chip,
                        radius: 8.0 * d,
                    });
                    let hist = &self.market_hist;
                    if hist.len() >= 2 {
                        let lo = hist.iter().map(|p| p.1).fold(f64::MAX, f64::min);
                        let hi = hist.iter().map(|p| p.1).fold(f64::MIN, f64::max);
                        let span = if hi > lo { hi - lo } else { hi.abs().max(1.0) * 0.02 };
                        let (y0, y1) = (lo - span * 0.08, hi + span * 0.08);
                        let (t0, t1) = (hist[0].0, hist[hist.len() - 1].0);
                        let inner = Rect::new(r.x + 8.0 * d, r.y + 26.0 * d, r.w - 16.0 * d, r.h - 50.0 * d);
                        let sx = |tv: f64| inner.x + ((tv - t0) / (t1 - t0).max(1.0)) as f32 * inner.w;
                        let sy = |v: f64| inner.y + inner.h - ((v - y0) / (y1 - y0)) as f32 * inner.h;
                        let mut lines: Vec<(u8, Vec<(f32, f32)>)> = vec![];
                        for k in 0..=3 {
                            let gy = inner.y + inner.h * k as f32 / 3.0;
                            lines.push((2, vec![(inner.x, gy), (inner.right(), gy)]));
                        }
                        // thin the series so very long histories stay quick to draw
                        let step = (hist.len() / 240).max(1);
                        let mut pts: Vec<(f32, f32)> = hist
                            .iter()
                            .step_by(step)
                            .map(|p| (sx(p.0), sy(p.1)))
                            .collect();
                        if let Some(l) = hist.last() {
                            pts.push((sx(l.0), sy(l.1)));
                        }
                        lines.push((1, pts));
                        let index = self.market_point.min(hist.len()-1);
                        let chosen = hist[index];
                        lines.push((0,vec![(sx(chosen.0),inner.y),(sx(chosen.0),inner.bottom())]));
                        out.push(W::Plot { r, lines });
                        let bins = hist.len().min(180);
                        for k in 0..bins {
                            let tv = t0 + (t1-t0)*(k as f64+0.5)/bins as f64;
                            let i = hist.iter().enumerate().min_by(|a,b| (a.1.0-tv).abs().total_cmp(&(b.1.0-tv).abs())).map(|p|p.0).unwrap_or(0);
                            out.push(W::Button {r:Rect::new(inner.x+inner.w*k as f32/bins as f32,inner.y,inner.w/bins as f32+1.0,inner.h),label:String::new(),px:0.0,kind:Kind::Invisible,action:Action::MarketPoint(i)});
                        }
                        let small = |out: &mut Vec<W>, rr: Rect, text: String, al: u8| {
                            out.push(W::Text {
                                r: rr,
                                text,
                                px: 11.0 * d,
                                bold: false,
                                c: t.dim,
                                align: al,
                                wrap: false,
                            });
                        };
                        small(out, Rect::new(r.x + 8.0 * d, r.y + 4.0 * d, r.w - 16.0 * d, 18.0 * d), format!("max {}", fmt_price(hi)), 0);
                        small(out, Rect::new(r.x + 8.0 * d, r.bottom() - 22.0 * d, r.w - 16.0 * d, 18.0 * d), fmt_ts(t0, self.market_range < 2), 0);
                        small(out, Rect::new(r.x + 8.0 * d, r.bottom() - 22.0 * d, r.w - 16.0 * d, 18.0 * d), format!("min {}", fmt_price(lo)), 1);
                        small(out, Rect::new(r.x + 8.0 * d, r.bottom() - 22.0 * d, r.w - 16.0 * d, 18.0 * d), fmt_ts(t1, self.market_range < 2), 2);
                        y += gh + 10.0 * d;
                        para(out, &mut y, &format!("{} UTC  |  {} {}", fmt_ts(chosen.0,true),fmt_price(chosen.1),q.currency),14.0*d);
                        chips(out,&mut y,&[("◀".into(),Action::MarketPoint(index.saturating_sub(1)),false),("▶".into(),Action::MarketPoint((index+1).min(hist.len()-1)),false)]);
                        para(out, &mut y, "Drag across the chart. Shows the nearest recorded sample, not every trade.",11.0*d);
                        let first = hist[0].1;
                        let last = hist[hist.len() - 1].1;
                        let pct = if first != 0.0 { (last / first - 1.0) * 100.0 } else { 0.0 };
                        row(out, &mut y, crate::i18n::t("Change in range"), &format!("{:+.2}%", pct), Action::None);
                        row(out, &mut y, crate::i18n::t("Highest"), &fmt_price(hi), Action::None);
                        row(out, &mut y, crate::i18n::t("Lowest"), &fmt_price(lo), Action::None);
                        row(out, &mut y, crate::i18n::t("Start"), &fmt_price(first), Action::None);
                    } else {
                        out.push(W::Text {
                            r: r.inset(12.0 * d, 12.0 * d),
                            text: self.market_hist_status.clone(),
                            px: 14.0 * d,
                            bold: false,
                            c: t.dim,
                            align: 1,
                            wrap: true,
                        });
                        y += gh + 10.0 * d;
                    }
                    for (label,value) in &q.info { row(out,&mut y,label,value,Action::None); }
                    para(out,&mut y,if self.market_kind == MarketKind::Crypto { "Source: CoinGecko. Market cap and volume in the quote currency. Day high/low unavailable from this endpoint." } else { "Source: Yahoo Finance. Volume in shares. Some fields may be unavailable. 7D uses the last 5 trading days." },11.0*d);
                    row(
                        out,
                        &mut y,
                        crate::i18n::t("Use price in the calculator"),
                        "›",
                        Action::UseValue(q.price),
                    );
                    para(out, &mut y, crate::i18n::t("Informational only, not financial advice. Data may be delayed."), 11.0 * d);
                }
            }
            Screen::Quiz => {
                chips(out,&mut y,&[("Arithmetic".into(),Action::QuizMode(0),self.quiz_mode==0),("Equations".into(),Action::QuizMode(1),self.quiz_mode==1),("Chemistry".into(),Action::QuizMode(2),self.quiz_mode==2)]);
                para(out,&mut y,&format!("Score: {} / {}   Best: {} / 10",self.quiz_score,self.quiz_count,self.s.quiz_best[self.quiz_mode]),17.0*d);
                if self.quiz_count>=10 {heading(out,&mut y,"Round complete!");}
                else if let Some(q)=&self.quiz_question {
                    para(out,&mut y,&q.text,22.0*d);
                    for (i,opt) in q.options.iter().enumerate(){row(out,&mut y,opt,if self.quiz_answered && i==q.answer {"✓"}else{""},if self.quiz_answered {Action::None}else{Action::QuizAnswer(i)});}
                }
                if !self.quiz_feedback.is_empty(){para(out,&mut y,&self.quiz_feedback,16.0*d);}
                if self.quiz_question.is_none() || self.quiz_answered || self.quiz_count>=10 {row(out,&mut y,if self.quiz_count>=10 {"New round"}else{"Next question"},"›",Action::Set("quiz_next"));}
            }
            Screen::Formulas => {
                para(out,&mut y,"Create formulas with named variables: width*height, mass*speed^2/2. Use explicit * for multiplication. sin, cos, sqrt and pi are supported. Angles use DEG.",14.0*d);
                row(out,&mut y,"Add formula","+",Action::Set("formula_new"));
                for (i,f) in self.s.formulas.iter().enumerate(){
                    heading(out,&mut y,&f.name);para(out,&mut y,&f.expression,17.0*d);
                    chips(out,&mut y,&[("Calculate".into(),Action::FormulaOpen(i),false),("Delete".into(),Action::FormulaDelete(i),false)]);
                }
                if !self.formula_result.is_empty(){heading(out,&mut y,"Result");para(out,&mut y,&self.formula_result,22.0*d);}
            }
            Screen::StatsCharts => {
                row(out,&mut y,"Enter data","›",Action::Set("stats_input"));
                row(out,&mut y,"Chart",if self.stats_pie {"Pie"}else{"Bars"},Action::Set("stats_pie"));
                if !self.stats_data.is_empty(){
                    let sum:f64=self.stats_data.iter().sum();let count=self.stats_data.len();
                    para(out,&mut y,&format!("n = {count}  Sum = {}\nMean = {}",display_num(sum),display_num(sum/count as f64)),16.0*d);
                    let r=Rect::new(x,y,w,w*0.85);
                    out.push(W::Fill{r,c:t.chip,radius:8.0*d});
                    let mut lines=vec![];
                    if self.stats_pie {
                        if self.stats_data.iter().any(|v|*v<0.) || sum<=0. {para(out,&mut y,"Pie charts need nonnegative values and a positive total. Use bars for negatives.",15.0*d);}
                        else {
                            let radius=w*0.34;let center=(r.x+r.w/2.,r.y+r.h/2.);let mut angle=-std::f32::consts::FRAC_PI_2;
                            for (i,v) in self.stats_data.iter().enumerate(){
                                let end=angle+(*v/sum) as f32*std::f32::consts::TAU;let steps=((end-angle)*radius).ceil().max(1.) as usize;
                                for k in 0..steps {let a=angle+(end-angle)*k as f32/steps as f32;lines.push((3+(i%6) as u8,vec![center,(center.0+radius*a.cos(),center.1+radius*a.sin())]));}
                                angle=end;
                            }
                        }
                    } else {
                        let lo=self.stats_data.iter().copied().fold(0.,f64::min);let hi=self.stats_data.iter().copied().fold(0.,f64::max);let span=(hi-lo).max(1.);
                        let yv=|v:f64|r.y+10.*d+((hi-v)/span) as f32*(r.h-25.*d);
                        lines.push((0,vec![(r.x,yv(0.)),(r.right(),yv(0.))]));
                        let bw=w/count as f32;
                        for (i,v) in self.stats_data.iter().enumerate(){let xx=x+bw*(i as f32+0.5);let color=[[45,125,230],[235,115,60],[60,165,90],[160,90,200],[225,70,100],[40,165,175]][i%6];
                            out.push(W::Fill {r:Rect::new(xx-bw*0.35,yv(*v).min(yv(0.)),bw*0.7,(yv(*v)-yv(0.)).abs().max(1.0)),c:color,radius:2.0*d});}
                    }
                    out.push(W::Plot{r,lines});y=r.bottom()+12.*d;
                    for (i,v) in self.stats_data.iter().enumerate(){row(out,&mut y,&format!("#{}",i+1),&if self.stats_pie && sum>0. {format!("{} ({:.1}%)",display_num(*v),v/sum*100.)}else{display_num(*v)},Action::None);}
                } else {para(out,&mut y,"Enter up to 60 numbers to plot. For example: 4 8 6 10.",16.0*d);}
            }
            Screen::Periodic => {
                for (i, e) in crate::science::ELEMENTS.iter().enumerate() {
                    row(
                        out,
                        &mut y,
                        &format!("{}  {}  {}", i + 1, e.0, e.1),
                        &expr::display_num(e.2),
                        Action::ElementOpen(i),
                    );
                }
            }
            Screen::Element => {
                let i = self.elem_sel.min(crate::science::ELEMENTS.len() - 1);
                let e = &crate::science::ELEMENTS[i];
                let z = i + 1;
                out.push(W::Text {
                    r: Rect::new(x, y, w, 80.0 * d),
                    text: e.0.to_string(),
                    px: 64.0 * d,
                    bold: true,
                    c: t.text,
                    align: 0,
                    wrap: false,
                });
                y += 84.0 * d;
                row(out, &mut y, crate::i18n::t("Atomic number"), &z.to_string(), Action::None);
                row(out, &mut y, crate::i18n::t("Atomic mass"), &expr::display_num(e.2), Action::UseValue(e.2));
                row(out, &mut y, crate::i18n::t("Category"), crate::i18n::t(crate::science::category_name(e.3)), Action::None);
                row(out, &mut y, crate::i18n::t("Period"), &crate::science::period_of(z).to_string(), Action::None);
                row(out, &mut y, crate::i18n::t("Protons / electrons"), &z.to_string(), Action::None);
                row(out, &mut y, crate::i18n::t("Neutrons (common isotope)"), &((e.2.round() as i64) - z as i64).max(0).to_string(), Action::None);
                heading(out, &mut y, crate::i18n::t("Electron configuration"));
                para(out, &mut y, &crate::science::electron_config(z), 15.0 * d);
                para(out, &mut y, crate::i18n::t("Tap the atomic mass to use it in the calculator (for molar masses). A few elements have slightly different real configurations."), 11.0 * d);
                let (pv, nx) = (i.saturating_sub(1), (i + 1).min(crate::science::ELEMENTS.len() - 1));
                chips(
                    out,
                    &mut y,
                    &[
                        ("‹".into(), Action::ElementOpen(pv), false),
                        ("›".into(), Action::ElementOpen(nx), false),
                    ],
                );
            }
            Screen::Constants => {
                para(out, &mut y, crate::i18n::t("Tap a constant to use its value in the calculator. They are also on scientific pages 5 and 6."), 12.0 * d);
                for c in crate::science::CONSTANTS {
                    let v: f64 = c.2.parse().unwrap_or(0.0);
                    row(
                        out,
                        &mut y,
                        &format!("{}  {}", c.0, crate::i18n::t(c.1)),
                        &format!("{} {}", c.2, c.3),
                        Action::UseValue(v),
                    );
                }
            }
            Screen::Steps => {
                let has_x = self.tokens.contains(&Tok::X);
                if !has_x {
                    para(out, &mut y, crate::i18n::t("Type an equation with x in the calculator (for example x² − 5x + 6 = 0 or 2x + 3 = 11), then open this screen."), 15.0 * d);
                } else {
                    for line in crate::steps::solve_steps(&self.tokens, self.s.degrees) {
                        let px = 15.0 * d;
                        let n = (line.chars().count() as f32 * px * 0.6 / (w - 4.0 * d)).ceil().max(1.0);
                        let hh = n * px * 1.4 + 8.0 * d;
                        out.push(W::Text {
                            r: Rect::new(x, y, w, hh),
                            text: line,
                            px,
                            bold: false,
                            c: t.text,
                            align: 0,
                            wrap: true,
                        });
                        y += hh + 6.0 * d;
                    }
                }
            }
            Screen::Graph3D => {
                let items: Vec<(String, Action, bool)> = crate::science::SURFACES
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (format!("z = {n}"), Action::Surface(i), i == self.g3_surface))
                    .collect();
                chips(out, &mut y, &items);
                let gh = w;
                let r = Rect::new(x, y, w, gh);
                out.push(W::Fill { r, c: t.chip, radius: 8.0 * d });
                let lines3 = crate::science::wireframe(self.g3_surface, self.g3_yaw, self.g3_pitch, self.g3_span);
                let (cx, cy, sc) = (r.x + r.w / 2.0, r.y + r.h / 2.0, r.w * 0.62);
                let lines: Vec<(u8, Vec<(f32, f32)>)> = lines3
                    .into_iter()
                    .map(|l| (1u8, l.into_iter().map(|(px, py)| (cx + px * sc, cy - py * sc)).collect()))
                    .collect();
                out.push(W::Plot { r, lines });
                y += gh + 8.0 * d;
                chips(
                    out,
                    &mut y,
                    &[
                        ("◀".into(), Action::Set("rot_l"), false),
                        ("▶".into(), Action::Set("rot_r"), false),
                        ("▲".into(), Action::Set("rot_u"), false),
                        ("▼".into(), Action::Set("rot_d"), false),
                        ("+".into(), Action::Set("g3zoom+"), false),
                        ("−".into(), Action::Set("g3zoom-"), false),
                    ],
                );
                para(out, &mut y, &crate::i18n::tf("x and y from {} to {}", &[&expr::display_num(-self.g3_span), &expr::display_num(self.g3_span)]), 12.0 * d);
            }
            Screen::Graph => {
                let toks = self.graph_tokens();
                if toks.is_empty() {
                    para(out, &mut y, crate::i18n::t("Type an expression with x in the calculator (e.g. x² − 2x − 3), then open Graph."), 15.0 * d);
                } else {
                    let ev = self.ev();
                    para(
                        out,
                        &mut y,
                        &format!("y = {}", display_tokens(&toks)),
                        16.0 * d,
                    );
                    let gh = w * 0.9;
                    let r = Rect::new(x, y, w, gh);
                    out.push(W::Fill {
                        r,
                        c: t.chip,
                        radius: 8.0 * d,
                    });
                    let span = self.graph_span;
                    let (x0, x1) = (-span, span);
                    let n = 400;
                    let mut ys = vec![];
                    for i in 0..=n {
                        let xv = x0 + (x1 - x0) * i as f64 / n as f64;
                        ys.push((xv, ev.f_at(&toks, xv)));
                    }
                    let fin: Vec<f64> = ys
                        .iter()
                        .filter_map(|p| p.1)
                        .filter(|v| v.is_finite())
                        .collect();
                    let (mut y0, mut y1) = (-span, span);
                    if !fin.is_empty() {
                        let mut sorted = fin.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let lo = sorted[sorted.len() / 20];
                        let hi = sorted[sorted.len() - 1 - sorted.len() / 20];
                        if hi > lo {
                            let m = (hi - lo) * 0.15;
                            y0 = lo - m;
                            y1 = hi + m;
                        }
                        if y0 > 0.0 {
                            y0 = -m1(y1);
                        }
                        if y1 < 0.0 {
                            y1 = m1(y0);
                        }
                    }
                    fn m1(v: f64) -> f64 {
                        v.abs() * 0.1
                    }
                    let sx = |xv: f64| r.x + ((xv - x0) / (x1 - x0)) as f32 * r.w;
                    let sy = |yv: f64| r.y + r.h - ((yv - y0) / (y1 - y0)) as f32 * r.h;
                    let mut lines: Vec<(u8, Vec<(f32, f32)>)> = vec![
                        (2, vec![(r.x, sy(0.0)), (r.right(), sy(0.0))]),
                        (2, vec![(sx(0.0), r.y), (sx(0.0), r.bottom())]),
                    ];
                    let mut cur: Vec<(f32, f32)> = vec![];
                    for (xv, yv) in &ys {
                        match yv {
                            Some(v)
                                if v.is_finite() && *v > y0 - (y1 - y0) && *v < y1 + (y1 - y0) =>
                            {
                                cur.push((sx(*xv), sy(*v)))
                            }
                            _ => {
                                if cur.len() > 1 {
                                    lines.push((1, std::mem::take(&mut cur)));
                                } else {
                                    cur.clear();
                                }
                            }
                        }
                    }
                    if cur.len() > 1 {
                        lines.push((1, cur));
                    }
                    out.push(W::Plot { r, lines });
                    out.push(W::Text {
                        r: Rect::new(r.x + 6.0 * d, r.y + 4.0 * d, r.w, 18.0 * d),
                        text: format!("y: {} … {}", display_num(y1), display_num(y0)),
                        px: 11.0 * d,
                        bold: false,
                        c: t.dim,
                        align: 0,
                        wrap: false,
                    });
                    out.push(W::Text {
                        r: Rect::new(r.x, r.bottom() - 20.0 * d, r.w - 6.0 * d, 18.0 * d),
                        text: format!("x: {} … {}", display_num(x0), display_num(x1)),
                        px: 11.0 * d,
                        bold: false,
                        c: t.dim,
                        align: 2,
                        wrap: false,
                    });
                    y += gh + 8.0 * d;
                    chips(
                        out,
                        &mut y,
                        &[
                            ("−".into(), Action::Set("gzoom-"), false),
                            ("+".into(), Action::Set("gzoom+"), false),
                        ],
                    );
                    let f = |xv: f64| ev.f_at(&toks, xv);
                    let roots = expr::numeric_roots(&f, x0, x1);
                    if !roots.is_empty() {
                        row(
                            out,
                            &mut y,
                            crate::i18n::t("Zeros"),
                            &roots
                                .iter()
                                .take(4)
                                .map(|v| display_num(*v))
                                .collect::<Vec<_>>()
                                .join("; "),
                            Action::None,
                        );
                    }
                    if let Some(v0) = f(0.0) {
                        row(out, &mut y, "y(0)", &display_num(v0), Action::UseValue(v0));
                    }
                    if let Ok(pl) = ev.eval_poly(&toks) {
                        row(out, &mut y, "f′(x)", &poly_text(&pl.deriv()), Action::None);
                        let mut ic = [0.0; 5];
                        for i in 0..4 {
                            ic[i + 1] = pl.0[i] / (i + 1) as f64;
                        }
                        row(
                            out,
                            &mut y,
                            "∫ f(x) dx",
                            &format!("{} + C", poly_text5(&ic)),
                            Action::None,
                        );
                    } else {
                        let dv = |xv: f64| Some((f(xv + 1e-5)? - f(xv - 1e-5)?) / 2e-5);
                        if let Some(v) = dv(1.0) {
                            row(
                                out,
                                &mut y,
                                "f′(1)",
                                &display_num((v * 1e6).round() / 1e6),
                                Action::None,
                            );
                        }
                    }
                    if let Some(iv) = expr::integrate(&f, 0.0, 1.0) {
                        row(
                            out,
                            &mut y,
                            "∫₀¹ f(x) dx",
                            &display_num((iv * 1e9).round() / 1e9),
                            Action::UseValue(iv),
                        );
                    }
                }
            }
            Screen::Photo => {
                let bw = (w - 10.0 * d) / 2.0;
                out.push(W::Button {
                    r: Rect::new(x, y, bw, 48.0 * d),
                    label: crate::i18n::t("Take photo").into(),
                    px: 16.0 * d,
                    kind: Kind::ChipOn,
                    action: Action::TakePhoto,
                });
                out.push(W::Button {
                    r: Rect::new(x + bw + 10.0 * d, y, bw, 48.0 * d),
                    label: crate::i18n::t("From gallery").into(),
                    px: 16.0 * d,
                    kind: Kind::Chip,
                    action: Action::OpenGallery,
                });
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
                if !self.gallery_status.is_empty() {
                    para(out, &mut y, &self.gallery_status, 14.0 * d);
                }
                let cols = 3usize;
                let gap = 6.0 * d;
                let cw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
                for (i, it) in self.gallery.iter().enumerate() {
                    let (c, r) = ((i % cols) as f32, (i / cols) as f32);
                    let rect = Rect::new(x + c * (cw + gap), y + r * (cw + gap), cw, cw);
                    out.push(W::Fill {
                        r: rect,
                        c: t.chip,
                        radius: 6.0 * d,
                    });
                    if it.thumb.is_some() {
                        out.push(W::Thumb { r: rect, idx: i });
                    } else {
                        out.push(W::Text {
                            r: rect.inset(6.0 * d, 0.0),
                            text: it.label.clone(),
                            px: 12.0 * d,
                            bold: false,
                            c: t.dim,
                            align: 1,
                            wrap: true,
                        });
                    }
                    out.push(W::Button {
                        r: rect,
                        label: String::new(),
                        px: 0.0,
                        kind: Kind::Invisible,
                        action: Action::GalleryPick(it.id),
                    });
                }
                y += self.gallery.len().div_ceil(cols) as f32 * (cw + gap);
            }
            _=>{}
        }
        y-area.y+24.0*d
    }
                             }
