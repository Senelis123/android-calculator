//! Software drawing: canvas, cached glyph rendering, rounded rectangles, images.

use fontdue::{Font, FontSettings, Metrics};
use std::cell::RefCell;
use std::collections::HashMap;

static REGULAR: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
static BOLD: &[u8] = include_bytes!("../fonts/DejaVuSans-Bold.ttf");

pub type Rgb = [u8; 3];

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self { Rect { x, y, w, h } }
    pub fn contains(&self, px: f32, py: f32) -> bool { px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h }
    pub fn inset(&self, dx: f32, dy: f32) -> Rect { Rect { x: self.x + dx, y: self.y + dy, w: (self.w - 2.0 * dx).max(0.0), h: (self.h - 2.0 * dy).max(0.0) } }
    pub fn bottom(&self) -> f32 { self.y + self.h }
    pub fn right(&self) -> f32 { self.x + self.w }
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x = self.x.max(o.x); let y = self.y.max(o.y);
        Rect { x, y, w: (self.right().min(o.right()) - x).max(0.0), h: (self.bottom().min(o.bottom()) - y).max(0.0) }
    }
}

pub struct Canvas<'a> { pub px: &'a mut [u8], pub w: usize, pub h: usize, pub stride: usize }

impl Canvas<'_> {
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || a <= 0.003 { return; }
        let i = (y as usize * self.stride + x as usize) * 4;
        if a >= 0.997 { self.px[i] = c[0]; self.px[i + 1] = c[1]; self.px[i + 2] = c[2]; self.px[i + 3] = 255; return; }
        for k in 0..3 { let d = self.px[i + k] as f32; self.px[i + k] = (d + (c[k] as f32 - d) * a) as u8; }
        self.px[i + 3] = 255;
    }
    pub fn fill(&mut self, c: Rgb) {
        for y in 0..self.h {
            let row = &mut self.px[y * self.stride * 4..(y * self.stride + self.w) * 4];
            for p in row.chunks_exact_mut(4) { p[0] = c[0]; p[1] = c[1]; p[2] = c[2]; p[3] = 255; }
        }
    }
    pub fn round_rect(&mut self, r: Rect, radius: f32, c: Rgb, clip: Rect) {
        let rr = r.intersect(&clip);
        if rr.w <= 0.0 || rr.h <= 0.0 { return; }
        let radius = radius.min(r.w / 2.0).min(r.h / 2.0);
        let (x0, y0, x1, y1) = (rr.x.floor() as i32, rr.y.floor() as i32, rr.right().ceil() as i32, rr.bottom().ceil() as i32);
        for y in y0..y1 {
            let cy = y as f32 + 0.5;
            let cov_y = ((cy - r.y + 0.5).min(r.bottom() - cy + 0.5)).clamp(0.0, 1.0);
            let qy = (r.y + radius - cy).max(cy - (r.bottom() - radius));
            for x in x0..x1 {
                let cx = x as f32 + 0.5;
                let cov_x = ((cx - r.x + 0.5).min(r.right() - cx + 0.5)).clamp(0.0, 1.0);
                let mut cov = cov_x * cov_y;
                let qx = (r.x + radius - cx).max(cx - (r.right() - radius));
                if qx > 0.0 && qy > 0.0 {
                    let d = (qx * qx + qy * qy).sqrt();
                    cov *= (radius - d + 0.5).clamp(0.0, 1.0);
                }
                self.blend(x, y, c, cov);
            }
        }
    }
    pub fn outline(&mut self, r: Rect, t: f32, c: Rgb, clip: Rect) {
        self.round_rect(Rect::new(r.x, r.y, r.w, t), 0.0, c, clip);
        self.round_rect(Rect::new(r.x, r.bottom() - t, r.w, t), 0.0, c, clip);
        self.round_rect(Rect::new(r.x, r.y, t, r.h), 0.0, c, clip);
        self.round_rect(Rect::new(r.right() - t, r.y, t, r.h), 0.0, c, clip);
    }
    /// Scales an RGB image into `r` (nearest neighbour, keeps aspect ratio, centered). Returns the drawn rect.
    pub fn image(&mut self, img: &image::RgbImage, r: Rect, clip: Rect) -> Rect {
        let (iw, ih) = (img.width() as f32, img.height() as f32);
        if iw < 1.0 || ih < 1.0 { return r; }
        let s = (r.w / iw).min(r.h / ih);
        let (dw, dh) = (iw * s, ih * s);
        let dst = Rect::new(r.x + (r.w - dw) / 2.0, r.y + (r.h - dh) / 2.0, dw, dh);
        let c = dst.intersect(&clip);
        for y in c.y as i32..c.bottom() as i32 {
            let sy = (((y as f32 + 0.5 - dst.y) / s) as u32).min(img.height() - 1);
            for x in c.x as i32..c.right() as i32 {
                let sx = (((x as f32 + 0.5 - dst.x) / s) as u32).min(img.width() - 1);
                let p = img.get_pixel(sx, sy).0;
                self.blend(x, y, p, 1.0);
            }
        }
        dst
    }
}

pub struct Fonts { regular: Font, bold: Font, cache: RefCell<HashMap<(char, u32, bool), (Metrics, Vec<u8>)>> }

impl Default for Fonts { fn default() -> Self { Self::new() } }

impl Fonts {
    pub fn new() -> Self {
        Fonts {
            regular: Font::from_bytes(REGULAR, FontSettings::default()).expect("font"),
            bold: Font::from_bytes(BOLD, FontSettings::default()).expect("font"),
            cache: RefCell::new(HashMap::new()),
        }
    }
    fn font(&self, bold: bool) -> &Font { if bold { &self.bold } else { &self.regular } }
    pub fn width(&self, s: &str, px: f32, bold: bool) -> f32 {
        let f = self.font(bold);
        s.chars().map(|ch| f.metrics(ch, px).advance_width).sum()
    }
    /// (ascent, descent (negative), line height)
    pub fn line(&self, px: f32, bold: bool) -> (f32, f32, f32) {
        let m = self.font(bold).horizontal_line_metrics(px).unwrap();
        (m.ascent, m.descent, m.new_line_size)
    }
    pub fn draw(&self, cv: &mut Canvas, s: &str, px: f32, bold: bool, x: f32, baseline: f32, c: Rgb, clip: Rect) {
        let mut pen = x;
        if self.cache.borrow().len() > 3000 { self.cache.borrow_mut().clear(); }
        for ch in s.chars() {
            let key = (ch, (px * 4.0) as u32, bold);
            let mut cache = self.cache.borrow_mut();
            let (m, bmp) = cache.entry(key).or_insert_with(|| self.font(bold).rasterize(ch, px));
            let gx = (pen + m.xmin as f32).round() as i32;
            let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
            for row in 0..m.height {
                let y = gy + row as i32;
                if (y as f32) < clip.y || (y as f32) >= clip.bottom() { continue; }
                for col in 0..m.width {
                    let x = gx + col as i32;
                    if (x as f32) < clip.x || (x as f32) >= clip.right() { continue; }
                    let a = bmp[row * m.width + col];
                    if a > 0 { cv.blend(x, y, c, a as f32 / 255.0); }
                }
            }
            pen += m.advance_width;
        }
    }
    /// Splits text into lines that fit `w` (breaks at spaces when possible, otherwise anywhere).
    pub fn wrap(&self, s: &str, px: f32, bold: bool, w: f32) -> Vec<String> {
        let mut lines = vec![];
        for para in s.split('\n') {
            let mut cur = String::new();
            for word in para.split_inclusive(' ') {
                let cand = format!("{cur}{word}");
                if self.width(cand.trim_end(), px, bold) <= w { cur = cand; continue; }
                if !cur.is_empty() { lines.push(cur.trim_end().to_string()); cur = String::new(); }
                // word longer than a line: break by characters
                for ch in word.chars() {
                    let cand = format!("{cur}{ch}");
                    if self.width(cand.trim_end(), px, bold) > w && !cur.is_empty() { lines.push(cur.trim_end().to_string()); cur = ch.to_string(); } else { cur = cand; }
                }
            }
            lines.push(cur.trim_end().to_string());
        }
        lines
    }
}
