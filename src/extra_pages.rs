//! Games hub, game screens, programmer mode and the formula sheet.
use super::*;
use crate::games::{self, Game};

pub const GAME_NAMES: [&str; games::GAME_COUNT] = ["2048", "15 puzzle", "Tetris", "Snake", "Minesweeper"];
pub const GAME_HINTS: [&str; games::GAME_COUNT] = [
    "Merge equal tiles and reach 2048",
    "Slide tiles into order 1 to 15",
    "Stack the falling pieces, clear lines",
    "Eat the red dots, avoid walls and yourself",
    "Open every cell without a mine",
];

fn tile_2048(v: u32) -> (Rgb, Rgb) {
    match v {
        0 => ([0xCD, 0xC1, 0xB4], [0x77, 0x6E, 0x65]),
        2 => ([0xEE, 0xE4, 0xDA], [0x77, 0x6E, 0x65]),
        4 => ([0xED, 0xE0, 0xC8], [0x77, 0x6E, 0x65]),
        8 => ([0xF2, 0xB1, 0x79], [0xFF, 0xFF, 0xFF]),
        16 => ([0xF5, 0x95, 0x63], [0xFF, 0xFF, 0xFF]),
        32 => ([0xF6, 0x7C, 0x5F], [0xFF, 0xFF, 0xFF]),
        64 => ([0xF6, 0x5E, 0x3B], [0xFF, 0xFF, 0xFF]),
        128 => ([0xED, 0xCF, 0x72], [0xFF, 0xFF, 0xFF]),
        256 => ([0xED, 0xCC, 0x61], [0xFF, 0xFF, 0xFF]),
        512 => ([0xED, 0xC8, 0x50], [0xFF, 0xFF, 0xFF]),
        1024 => ([0xED, 0xC5, 0x3F], [0xFF, 0xFF, 0xFF]),
        2048 => ([0xED, 0xC2, 0x2E], [0xFF, 0xFF, 0xFF]),
        _ => ([0x3C, 0x3A, 0x32], [0xFF, 0xFF, 0xFF]),
    }
}

const PIECE_COLORS: [Rgb; 8] = [
    [0x26, 0x26, 0x2E], // empty (unused)
    [0x00, 0xC8, 0xDC], // I
    [0xE6, 0xC8, 0x28], // O
    [0xA0, 0x5A, 0xC8], // T
    [0x3C, 0xB4, 0x5A], // S
    [0xDC, 0x3C, 0x3C], // Z
    [0x3C, 0x6E, 0xDC], // J
    [0xE6, 0x8C, 0x28], // L
];

const MINE_NUM_COLORS: [Rgb; 9] = [
    [0x00, 0x00, 0x00],
    [0x32, 0x6C, 0xD6], [0x2E, 0x8B, 0x57], [0xD3, 0x2F, 0x2F], [0x1A, 0x3A, 0x8F],
    [0x8B, 0x1A, 0x1A], [0x1A, 0x8B, 0x8B], [0x20, 0x20, 0x20], [0x60, 0x60, 0x60],
];

impl App {
    pub(super) fn build_extra_page(&self, out: &mut Vec<W>, area: Rect, d: f32, t: &Theme) -> f32 {
        let pad = 16.0 * d;
        let x = area.x + pad;
        let w = area.w - 2.0 * pad;
        let mut y = area.y + 8.0 * d;
        let text = |out: &mut Vec<W>, r: Rect, s: String, px: f32, bold: bool, c: Rgb, align: u8| {
            out.push(W::Text { r, text: s, px, bold, c, align, wrap: false });
        };
        match self.screen {
            Screen::Games => {
                for i in 0..games::GAME_COUNT {
                    let r = Rect::new(x, y, w, 64.0 * d);
                    out.push(W::Button { r, label: String::new(), px: 0.0, kind: Kind::Row, action: Action::GameNav(i) });
                    text(out, Rect::new(r.x + 14.0 * d, r.y + 8.0 * d, r.w * 0.6, 26.0 * d), crate::i18n::t(GAME_NAMES[i]).to_string(), 17.0 * d, true, t.text, 0);
                    text(out, Rect::new(r.x + 14.0 * d, r.y + 34.0 * d, r.w - 28.0 * d, 22.0 * d), crate::i18n::t(GAME_HINTS[i]).to_string(), 12.5 * d, false, t.dim, 0);
                    let best = self.s.game_best[i];
                    let bt = match i {
                        1 => {
                            if best == 0 { String::new() } else { crate::i18n::tf("Best: {} moves", &[&best]) }
                        }
                        4 => {
                            if best == 0 { String::new() } else { crate::i18n::tf("Wins: {}", &[&best]) }
                        }
                        _ => {
                            if best == 0 { String::new() } else { crate::i18n::tf("Best: {}", &[&best]) }
                        }
                    };
                    text(out, Rect::new(r.x + r.w * 0.55, r.y + 8.0 * d, r.w * 0.45 - 14.0 * d, 26.0 * d), bt, 13.0 * d, false, t.chip_on, 2);
                    y = r.bottom() + 8.0 * d;
                }
                y += 8.0 * d;
            }
            Screen::Game(kind) => {
                let bw = (w - 8.0 * d) / 2.0;
                out.push(W::Button {
                    r: Rect::new(x, y, bw, 42.0 * d),
                    label: crate::i18n::t("New game").into(),
                    px: 15.0 * d,
                    kind: Kind::ChipOn,
                    action: Action::GameNew,
                });
                if kind == 4 {
                    out.push(W::Button {
                        r: Rect::new(x + bw + 8.0 * d, y, bw, 42.0 * d),
                        label: crate::i18n::t("Flag mode").into(),
                        px: 15.0 * d,
                        kind: if self.mines_flag { Kind::ChipOn } else { Kind::Chip },
                        action: Action::Set("mines_flag"),
                    });
                }
                y += 50.0 * d;
                match &self.game {
                    Some(Game::G2048(g)) => {
                        text(out, Rect::new(x, y, w, 26.0 * d),
                            format!("{}   {}", crate::i18n::tf("Score: {}", &[&g.score]), crate::i18n::tf("Best: {}", &[&self.s.game_best[0]])),
                            16.0 * d, true, t.text, 0);
                        y += 32.0 * d;
                        let gap = 8.0 * d;
                        let cell = (w - 5.0 * gap) / 4.0;
                        out.push(W::Fill { r: Rect::new(x, y, w, w), c: t.chip, radius: 12.0 * d });
                        for yy in 0..4 {
                            for xx in 0..4 {
                                let v = g.g[yy][xx];
                                let (bg, fg) = tile_2048(v);
                                let r = Rect::new(x + gap + xx as f32 * (cell + gap), y + gap + yy as f32 * (cell + gap), cell, cell);
                                out.push(W::Fill { r, c: bg, radius: 8.0 * d });
                                if v != 0 {
                                    let px = if v < 10 { 30.0 * d } else if v < 100 { 26.0 * d } else if v < 1000 { 21.0 * d } else { 17.0 * d };
                                    text(out, r, v.to_string(), px, true, fg, 1);
                                }
                            }
                        }
                        y += w + 10.0 * d;
                        if g.won { text(out, Rect::new(x, y, w, 28.0 * d), crate::i18n::t("You reached 2048!").into(), 17.0 * d, true, t.chip_on, 1); y += 34.0 * d; }
                        if g.over { text(out, Rect::new(x, y, w, 28.0 * d), crate::i18n::t("Game over").into(), 17.0 * d, true, t.text, 1); y += 34.0 * d; }
                        y = self.dpad(out, x, y, w, d, &[2, 0, 3, 1]);
                    }
                    Some(Game::Fifteen(f)) => {
                        text(out, Rect::new(x, y, w, 26.0 * d),
                            crate::i18n::tf("Moves: {}", &[&f.moves]), 16.0 * d, true, t.text, 0);
                        y += 32.0 * d;
                        let gap = 8.0 * d;
                        let cell = (w - 5.0 * gap) / 4.0;
                        for i in 0..16 {
                            let (cx, cy) = (i % 4, i / 4);
                            let r = Rect::new(x + gap + cx as f32 * (cell + gap), y + gap + cy as f32 * (cell + gap), cell, cell);
                            let v = f.b[i];
                            if v == 0 {
                                out.push(W::Fill { r, c: t.chip, radius: 8.0 * d });
                            } else {
                                out.push(W::Button { r, label: v.to_string(), px: 26.0 * d, kind: Kind::Digit, action: Action::GameTap(i) });
                            }
                        }
                        y += 5.0 * gap + 4.0 * cell + 10.0 * d;
                        if f.solved() {
                            text(out, Rect::new(x, y, w, 30.0 * d), crate::i18n::tf("Solved in {} moves!", &[&f.moves]), 18.0 * d, true, t.chip_on, 1);
                            y += 36.0 * d;
                        } else {
                            text(out, Rect::new(x, y, w, 22.0 * d), crate::i18n::t("Tap a tile next to the empty space").into(), 13.0 * d, false, t.dim, 1);
                            y += 28.0 * d;
                        }
                    }
                    Some(Game::Tetris(tt)) => {
                        let s = (w / 15.5).floor();
                        let (bw, bh) = (10.0 * s, 20.0 * s);
                        let br = Rect::new(x, y, bw, bh);
                        out.push(W::Fill { r: br, c: t.chip, radius: 6.0 * d });
                        let view = tt.view();
                        for yy in 0..games::TH {
                            for xx in 0..games::TW {
                                let c = view[yy][xx];
                                if c != 0 {
                                    let r = Rect::new(x + xx as f32 * s + 1.0, y + yy as f32 * s + 1.0, s - 2.0, s - 2.0);
                                    out.push(W::Fill { r, c: PIECE_COLORS[c as usize], radius: 2.0 * d });
                                }
                            }
                        }
                        let px0 = br.right() + 10.0 * d;
                        let pw = w - bw - 10.0 * d;
                        text(out, Rect::new(px0, y, pw, 22.0 * d), crate::i18n::tf("Score: {}", &[&tt.score]), 14.0 * d, true, t.text, 0);
                        text(out, Rect::new(px0, y + 26.0 * d, pw, 20.0 * d), crate::i18n::tf("Lines: {}", &[&tt.lines]), 13.0 * d, false, t.dim, 0);
                        text(out, Rect::new(px0, y + 48.0 * d, pw, 20.0 * d), crate::i18n::tf("Level: {}", &[&tt.level]), 13.0 * d, false, t.dim, 0);
                        text(out, Rect::new(px0, y + 68.0 * d, pw, 20.0 * d), crate::i18n::tf("Best: {}", &[&self.s.game_best[2]]), 13.0 * d, false, t.dim, 0);
                        text(out, Rect::new(px0, y + 96.0 * d, pw, 20.0 * d), crate::i18n::t("Next:").into(), 13.0 * d, false, t.dim, 0);
                        let ns = 7.0 * d;
                        for (xx, yy) in crate::games::shape_cells(tt.next, 0) {
                            let r = Rect::new(px0 + 6.0 * d + xx as f32 * ns, y + 118.0 * d + yy as f32 * ns, ns - 1.5, ns - 1.5);
                            out.push(W::Fill { r, c: PIECE_COLORS[tt.next + 1], radius: 1.5 * d });
                        }
                        if tt.over {
                            text(out, Rect::new(x, y + bh / 2.0 - 16.0 * d, bw, 32.0 * d), crate::i18n::t("Game over").into(), 19.0 * d, true, t.text, 1);
                        }
                        y += bh + 12.0 * d;
                        // controls: left, right, rotate / soft drop, hard drop
                        let keys: [(&str, u8); 5] = [("◀", 0), ("▶", 1), ("↻", 2), ("▼", 3), ("⇊", 4)];
                        let gap = 6.0 * d;
                        let kw = (w - 4.0 * gap) / 5.0;
                        for (i, (label, code)) in keys.iter().enumerate() {
                            out.push(W::Button {
                                r: Rect::new(x + i as f32 * (kw + gap), y, kw, 52.0 * d),
                                label: label.to_string(),
                                px: 20.0 * d,
                                kind: Kind::Sci,
                                action: Action::GameMove(*code),
                            });
                        }
                        y += 62.0 * d;
                    }
                    Some(Game::Snake(sn)) => {
                        text(out, Rect::new(x, y, w, 26.0 * d),
                            format!("{}   {}", crate::i18n::tf("Score: {}", &[&sn.score]), crate::i18n::tf("Best: {}", &[&self.s.game_best[3]])),
                            16.0 * d, true, t.text, 0);
                        y += 32.0 * d;
                        let s = (w / (games::SW as f32 + 2.0)).floor();
                        let br = Rect::new(x, y, games::SW as f32 * s, games::SH as f32 * s);
                        out.push(W::Fill { r: br, c: t.chip, radius: 6.0 * d });
                        let fr = Rect::new(br.x + sn.food.0 as f32 * s + s * 0.15, br.y + sn.food.1 as f32 * s + s * 0.15, s * 0.7, s * 0.7);
                        out.push(W::Fill { r: fr, c: [0xE0, 0x3C, 0x3C], radius: s * 0.35 });
                        for (i, &(bx, by)) in sn.body.iter().enumerate() {
                            let r = Rect::new(br.x + bx as f32 * s + 1.0, br.y + by as f32 * s + 1.0, s - 2.0, s - 2.0);
                            let c = if i == 0 { [0x8C, 0xE0, 0x5A] } else { [0x3C, 0xB4, 0x5A] };
                            out.push(W::Fill { r, c, radius: 2.0 * d });
                        }
                        if sn.over {
                            text(out, Rect::new(br.x, br.y + br.h / 2.0 - 16.0 * d, br.w, 32.0 * d), crate::i18n::t("Game over").into(), 19.0 * d, true, t.text, 1);
                        }
                        y = br.bottom() + 12.0 * d;
                        y = self.dpad(out, x, y, w, d, &[0, 3, 2, 1]);
                    }
                    Some(Game::Mines(m)) => {
                        text(out, Rect::new(x, y, w, 26.0 * d),
                            crate::i18n::tf("Flags left: {}", &[&m.flags_left()]), 15.0 * d, true, t.text, 0);
                        y += 32.0 * d;
                        let gap = 3.0 * d;
                        let cell = (w - 10.0 * gap) / 9.0;
                        for yy in 0..games::MH {
                            for xx in 0..games::MW {
                                let i = yy * games::MW + xx;
                                let r = Rect::new(x + gap + xx as f32 * (cell + gap), y + gap + yy as f32 * (cell + gap), cell, cell);
                                if m.open[yy][xx] {
                                    out.push(W::Fill { r, c: t.bg, radius: 3.0 * d });
                                    let n = m.count(xx, yy);
                                    let label = if m.over && !m.won && m.revealed_mine(xx, yy) { "✸".to_string() }
                                        else if n > 0 { n.to_string() } else { String::new() };
                                    if !label.is_empty() {
                                        text(out, r, label, 15.0 * d, true, MINE_NUM_COLORS[n as usize], 1);
                                    }
                                } else {
                                    let label = if m.flag[yy][xx] { "⚑" } else { "" };
                                    out.push(W::Button { r, label: label.into(), px: 15.0 * d, kind: Kind::Digit, action: Action::GameTap(i) });
                                }
                            }
                        }
                        y += 10.0 * gap + 9.0 * cell + 12.0 * d;
                        if m.over {
                            let msg = if m.won { crate::i18n::t("You win!") } else { crate::i18n::t("Game over") };
                            text(out, Rect::new(x, y, w, 30.0 * d), msg.into(), 18.0 * d, true, if m.won { t.chip_on } else { t.text }, 1);
                            y += 36.0 * d;
                        }
                    }
                    None => {}
                }
            }
            Screen::Prog => {
                let p = &self.prog;
                // base chips
                let bases: [(u8, &str); 4] = [(2, "BIN"), (8, "OCT"), (10, "DEC"), (16, "HEX")];
                let gap = 8.0 * d;
                let cw = (w - 3.0 * gap) / 4.0;
                for (i, (b, name)) in bases.iter().enumerate() {
                    out.push(W::Button {
                        r: Rect::new(x + i as f32 * (cw + gap), y, cw, 38.0 * d),
                        label: name.to_string(),
                        px: 14.0 * d,
                        kind: if p.base == *b { Kind::ChipOn } else { Kind::Chip },
                        action: Action::Set(match b { 2 => "prog_b2", 8 => "prog_b8", 10 => "prog_b10", _ => "prog_b16" }),
                    });
                }
                y += 48.0 * d;
                // display
                let shown = if p.error { crate::i18n::t("Error").to_string() } else { p.text.clone() };
                out.push(W::Text { r: Rect::new(x, y, w, 44.0 * d), text: shown, px: 30.0 * d, bold: true, c: t.text, align: 2, wrap: false });
                y += 50.0 * d;
                for (label, val) in p.conversions() {
                    out.push(W::Text { r: Rect::new(x, y, 60.0 * d, 22.0 * d), text: label, px: 12.0 * d, bold: true, c: t.dim, align: 0, wrap: false });
                    out.push(W::Text { r: Rect::new(x + 60.0 * d, y, w - 60.0 * d, 22.0 * d), text: val, px: 14.0 * d, bold: false, c: t.text, align: 2, wrap: false });
                    y += 26.0 * d;
                }
                y += 10.0 * d;
                // keypad
                let hex = p.base == 16;
                let digits: Vec<&str> = (0..p.base.min(10)).map(|n| match n { 0 => "0", 1 => "1", 2 => "2", 3 => "3", 4 => "4", 5 => "5", 6 => "6", 7 => "7", 8 => "8", _ => "9" }).collect();
                let mut rows: Vec<Vec<(&str, Kind)>> = vec![];
                if hex {
                    rows.push(vec![("A", Kind::Digit), ("B", Kind::Digit), ("C", Kind::Digit), ("D", Kind::Digit), ("E", Kind::Digit), ("F", Kind::Digit)]);
                }
                rows.push(vec![("AND", Kind::Sci), ("OR", Kind::Sci), ("XOR", Kind::Sci), ("NOT", Kind::Sci), ("<<", Kind::Sci), (">>", Kind::Sci)]);
                rows.push(vec![("÷", Kind::Op), ("×", Kind::Op), ("−", Kind::Op), ("+", Kind::Op), ("⌫", Kind::Op), (if hex { "AC" } else { "C" }, Kind::Op)]);
                let mut cur: Vec<(&str, Kind)> = digits.iter().map(|d| (*d, Kind::Digit)).collect();
                cur.push(("=", Kind::Eq));
                while !cur.len().is_multiple_of(6) { cur.push(("", Kind::Invisible)); }
                let chunk = cur.len() / 6;
                for part in cur.chunks(chunk.max(1)) { rows.push(part.to_vec()); }
                let kh = 52.0 * d;
                let kg = 6.0 * d;
                for row in &rows {
                    let n = row.len() as f32;
                    let kw = (w - (n - 1.0) * kg) / n;
                    for (i, (label, kind)) in row.iter().enumerate() {
                        if *kind == Kind::Invisible { continue; }
                        out.push(W::Button {
                            r: Rect::new(x + i as f32 * (kw + kg), y, kw, kh),
                            label: label.to_string(),
                            px: 18.0 * d,
                            kind: *kind,
                            action: Action::ProgKey(label),
                        });
                    }
                    y += kh + kg;
                }
            }
            Screen::Cheat => {
                for sec in crate::cheatsheet::SECTIONS {
                    y += 8.0 * d;
                    out.push(W::Text { r: Rect::new(x, y, w, 28.0 * d), text: crate::cheatsheet::title(sec).into(), px: 17.0 * d, bold: true, c: t.chip_on, align: 0, wrap: false });
                    y += 34.0 * d;
                    for line in sec.lines {
                        let s = crate::cheatsheet::line(line);
                        let px = 14.0 * d;
                        let n = (s.chars().count() as f32 * px * 0.6 / (w - 4.0 * d)).ceil().max(1.0);
                        let hh = n * px * 1.4 + 6.0 * d;
                        out.push(W::Text { r: Rect::new(x, y, w, hh), text: s.into(), px, bold: false, c: t.text, align: 0, wrap: true });
                        y += hh + 4.0 * d;
                    }
                }
            }
            _ => {}
        }
        y - area.y + 24.0 * d
    }

    /// Direction pad: [up, left, down, right] codes. Returns the new y.
    fn dpad(&self, out: &mut Vec<W>, x: f32, y: f32, w: f32, d: f32, codes: &[u8; 4]) -> f32 {
        let bw = w / 3.0 - 8.0 * d;
        let bh = 56.0 * d;
        let cx = x + w / 2.0;
        let place = |out: &mut Vec<W>, label: &str, code: u8, bx: f32, by: f32| {
            out.push(W::Button {
                r: Rect::new(bx - bw / 2.0, by, bw, bh),
                label: label.into(),
                px: 22.0 * d,
                kind: Kind::Sci,
                action: Action::GameMove(code),
            });
        };
        place(out, "▲", codes[0], cx, y);
        place(out, "◀", codes[1], cx - w / 3.0, y + bh + 6.0 * d);
        place(out, "▼", codes[2], cx, y + bh + 6.0 * d);
        place(out, "▶", codes[3], cx + w / 3.0, y + bh + 6.0 * d);
        y + 2.0 * bh + 18.0 * d
    }

}
