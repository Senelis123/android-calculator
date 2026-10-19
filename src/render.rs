use super::*;
pub fn draw(
    app: &App,
    frame: &Frame,
    fonts: &Fonts,
    cv: &mut Canvas,
    d: f32,
    pressed: Option<usize>,
) {
    let t = theme(app.dark());
    let full = Rect::new(0.0, 0.0, cv.w as f32, cv.h as f32);
    for (i, wd) in frame.widgets.iter().enumerate() {
        let clip = match frame.scroll_area {
            Some(a) if i >= frame.page.0 && i < frame.page.1 => a,
            _ => full,
        };
        let is_pressed = pressed == Some(i);
        match wd {
            W::Fill { r, c, radius } => cv.round_rect(*r, *radius, *c, clip),
            W::Text {
                r,
                text,
                px,
                bold,
                c,
                align,
                wrap,
            } => text_in(cv, fonts, *r, text, *px, *bold, *c, *align, *wrap, clip),
            W::Button {
                r, label, px, kind, ..
            } => {
                let (bg, fg, radius, shadow) = match kind {
                    Kind::Digit => (t.key, t.key_text, 14.0 * d, true),
                    Kind::Op => (t.key_op, t.key_text, 14.0 * d, true),
                    Kind::Sci => (t.key_op, t.key_text, 12.0 * d, false),
                    Kind::Eq => (t.key_eq, t.key_eq_text, 16.0 * d, true),
                    Kind::Flat => (t.bg, t.key_text, 8.0 * d, false),
                    Kind::Chip => (t.chip, t.key_text, r.h / 2.0, false),
                    Kind::ChipOn => (t.chip_on, t.chip_on_text, r.h / 2.0, false),
                    Kind::Row => (t.chip, t.key_text, 12.0 * d, false),
                    Kind::Invisible => (t.bg, t.key_text, 0.0, false),
                };
                if *kind == Kind::Invisible {
                    if is_pressed {
                        cv.round_rect(*r, 8.0 * d, t.line, clip);
                    }
                    continue;
                }
                let mut face = if matches!(kind, Kind::Digit | Kind::Op | Kind::Eq | Kind::Sci) {
                    r.inset(2.0 * d, 3.0 * d)
                } else {
                    *r
                };
                if is_pressed && app.s.animations {
                    face = face.inset(face.w * 0.025, face.h * 0.035);
                }
                if shadow {
                    cv.round_rect(
                        Rect {
                            y: face.y + 1.0 * d,
                            ..face
                        },
                        radius,
                        t.shadow,
                        clip,
                    );
                }
                let bg = if is_pressed { t.pressed } else { bg };
                cv.round_rect(face, radius, bg, clip);
                if !label.is_empty() {
                    let mut px = *px;
                    let tw = fonts.width(label, px, false);
                    if tw > face.w - 8.0 * d {
                        px *= (face.w - 8.0 * d) / tw;
                    }
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
                        let rr = Rect::new(
                            sx + b[0] * k * s - 4.0 * d,
                            sy + b[1] * k * s - 4.0 * d,
                            (b[2] - b[0]) * k * s + 8.0 * d,
                            (b[3] - b[1]) * k * s + 8.0 * d,
                        );
                        cv.outline(rr, 3.0 * d, [0xFF, 0x3B, 0x30], clip);
                    }
                }
            }
            W::Plot { r, lines } => {
                let c = |k: u8| match k {
                    0 => t.text,
                    1 => [0x1E, 0x88, 0xE5],
                    3 => [45,125,230],4 => [235,115,60],5 => [60,165,90],6 => [160,90,200],7 => [225,70,100],8 => [40,165,175],
                    _ => t.line,
                };
                let clip2 = r.intersect(&clip);
                for (k, pts) in lines {
                    let col = c(*k);
                    let th = if *k == 2 { 1.0 * d } else { 2.2 * d };
                    for w2 in pts.windows(2) {
                        let ((x0, y0), (x1, y1)) = (w2[0], w2[1]);
                        let n = ((x1 - x0).abs().max((y1 - y0).abs()) / (0.7 * d))
                            .ceil()
                            .max(1.0) as i32;
                        for i in 0..=n {
                            let f = i as f32 / n as f32;
                            let (px, py) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                            let rr = Rect::new(px - th / 2.0, py - th / 2.0, th, th);
                            let ri = rr.intersect(&clip2);
                            if ri.w > 0.0 && ri.h > 0.0 {
                                cv.round_rect(ri, 0.0, col, clip2);
                            }
                        }
                    }
                }
            }
            W::Thumb { r, idx } => {
                if let Some(Some(th)) = app.gallery.get(*idx).map(|g| g.thumb.as_ref()) {
                    cv.image(th, r.inset(2.0 * d, 2.0 * d), clip);
                }
            }
        }
    }
}

/// Width of the image the OCR ran on (the bbox coordinates refer to it).
pub static PHOTO_SRC_W: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
fn app_photo_src_w(_app: &App) -> f32 {
    PHOTO_SRC_W.load(std::sync::atomic::Ordering::Relaxed) as f32
}

pub fn text_in(
    cv: &mut Canvas,
    fonts: &Fonts,
    r: Rect,
    text: &str,
    px: f32,
    bold: bool,
    c: Rgb,
    align: u8,
    wrap: bool,
    clip: Rect,
) {
    if text.is_empty() {
        return;
    }
    let clip = clip.intersect(&r.inset(-2.0, -2.0));
    // user / system text size, but never taller than the box
    let mut px = (px * font_scale()).min(r.h.max(px));
    let mut lines = vec![text.to_string()];
    if wrap {
        // shrink down to 60 % before wrapping (keeps long numbers on one line when possible)
        let w0 = fonts.width(text, px, bold);
        if w0 > r.w && text.chars().count() <= 40 {
            px = (px * r.w / w0).max(px * 0.6);
        }
        lines = fonts.wrap(text, px, bold, r.w);
    } else {
        let w0 = fonts.width(text, px, bold);
        if w0 > r.w && (align != 0 || font_scale() > 1.0) {
            px *= r.w / w0;
        }
    }
    let (asc, desc, lh) = fonts.line(px, bold);
    let total = lh * (lines.len() as f32 - 1.0) + (asc - desc);
    let mut base = r.y + (r.h - total) / 2.0 + asc;
    if wrap && total > r.h {
        base = r.bottom() - total + asc;
    } // keep the end (latest digits) visible
    for line in &lines {
        let w = fonts.width(line, px, bold);
        let x = match align {
            0 => r.x,
            1 => r.x + (r.w - w) / 2.0,
            _ => r.right() - w,
        };
        fonts.draw(cv, line, px, bold, x, base, c, clip);
        base += lh;
    }
}
