//! On-device recognition of printed math problems from a photo (ocrs + rten, fully offline),
//! and conversion of the recognized text into calculator tokens.

use crate::expr::Tok;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use rten::Model;
use std::sync::OnceLock;

static DETECTION: &[u8] = include_bytes!("../models/text-detection.rten");
static RECOGNITION: &[u8] = include_bytes!("../models/text-recognition.rten");
/// Characters the recognizer may output. Restricting it avoids reading "1" as "l", "0" as "O" etc.
const ALLOWED: &str = "0123456789+-*/:=().,xX^% ";

fn engine() -> anyhow::Result<&'static OcrEngine> {
    static ENGINE: OnceLock<OcrEngine> = OnceLock::new();
    if let Some(e) = ENGINE.get() { return Ok(e); }
    let e = OcrEngine::new(OcrEngineParams {
        detection_model: Some(Model::load_static_slice(DETECTION)?),
        recognition_model: Some(Model::load_static_slice(RECOGNITION)?),
        allowed_chars: Some(ALLOWED.to_string()),
        ..Default::default()
    })?;
    Ok(ENGINE.get_or_init(|| e))
}

/// Downscales very large photos (camera images are 12+ MP) to keep memory and time reasonable.
pub fn prepare(img: image::DynamicImage) -> image::RgbImage {
    let (w, h) = (img.width(), img.height());
    let max = 1600u32;
    let img = if w.max(h) > max { img.resize(max, max, image::imageops::FilterType::Triangle) } else { img };
    img.into_rgb8()
}

#[derive(Debug, Clone)]
pub struct Line { pub text: String, /// left, top, right, bottom in image pixels
    pub bbox: [f32; 4] }

/// Returns the recognized text lines (with their boxes), top to bottom.
pub fn recognize_lines(img: &image::RgbImage) -> anyhow::Result<Vec<Line>> {
    let engine = engine()?;
    let src = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;
    let input = engine.prepare_input(src)?;
    let words = engine.detect_words(&input)?;
    let lines = engine.find_text_lines(&input, &words);
    let texts = engine.recognize_text(&input, &lines)?;
    Ok(texts.iter().flatten().filter_map(|l| {
        let text = l.to_string().trim().to_string();
        if !text.chars().any(|c| c.is_ascii_digit() || c == 'x' || c == 'X') { return None; }
        let r = l.bounding_rect();
        Some(Line { text, bbox: [r.left() as f32, r.top() as f32, r.right() as f32, r.bottom() as f32] })
    }).collect())
}

/// Returns the recognized text lines, top to bottom.
pub fn recognize(img: &image::RgbImage) -> anyhow::Result<Vec<String>> {
    let engine = engine()?;
    let src = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;
    let input = engine.prepare_input(src)?;
    let words = engine.detect_words(&input)?;
    let lines = engine.find_text_lines(&input, &words);
    let texts = engine.recognize_text(&input, &lines)?;
    Ok(texts.iter().flatten().map(|l| l.to_string().trim().to_string()).filter(|s| s.chars().any(|c| c.is_ascii_digit() || c == 'x')).collect())
}

/// Index of the line that looks most like a math problem.
pub fn best_index(lines: &[String]) -> Option<usize> {
    (0..lines.len()).max_by_key(|&i| score(&lines[i]))
}

fn score(l: &str) -> i32 {
    let ops = l.chars().filter(|c| "+-*/:=^xX".contains(*c)).count();
    let digits = l.chars().filter(|c| c.is_ascii_digit()).count();
    (ops.min(4) * 10 + digits.min(20)) as i32
}

/// Picks the line that looks most like a math problem.
pub fn best_line(lines: &[String]) -> Option<String> {
    lines.iter().max_by_key(|l| {
        let ops = l.chars().filter(|c| "+-*/:=^x".contains(*c)).count();
        let digits = l.chars().filter(|c| c.is_ascii_digit()).count();
        (ops.min(4) * 10 + digits.min(20)) as i32
    }).cloned()
}

/// Converts recognized text like "2x + 3 = 11", "12,5 : 5", "3 x 4 =" or "x^2-5x+6=0" into tokens.
/// An "x" between two numbers is read as multiplication when there is no equation, otherwise as the unknown.
pub fn to_tokens(text: &str) -> Vec<Tok> {
    let s: Vec<char> = text.chars().map(|c| if c == 'X' { 'x' } else { c }).collect();
    let eq_count = s.iter().filter(|c| **c == '=').count();
    // "12 + 7 =" (trailing '=') is a calculation, not an equation.
    let last_non_space = s.iter().rposition(|c| !c.is_whitespace());
    let trailing_eq = last_non_space.map(|i| s[i] == '=').unwrap_or(false);
    let is_equation = eq_count == 1 && !trailing_eq;
    let mut out: Vec<Tok> = vec![];
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c.is_ascii_digit() || ((c == ',' || c == '.') && s.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
            let mut num = String::new();
            while i < s.len() && (s[i].is_ascii_digit() || s[i] == ',' || s[i] == '.') {
                num.push(if s[i] == ',' { '.' } else { s[i] });
                i += 1;
            }
            // keep only the first decimal point
            let mut seen = false;
            let num: String = num.chars().filter(|&ch| if ch == '.' { let ok = !seen; seen = true; ok } else { true }).collect();
            let num = num.trim_end_matches('.').to_string();
            if num.starts_with('.') { out.push(Tok::Num(format!("0{num}"))); } else { out.push(Tok::Num(num)); }
            continue;
        }
        match c {
            '+' => out.push(Tok::Op('+')),
            '-' => out.push(Tok::Op('−')),
            '*' => out.push(Tok::Op('×')),
            '/' | ':' => out.push(Tok::Op('÷')),
            '^' => out.push(Tok::Op('^')),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '%' => out.push(Tok::Post("%")),
            '=' => if is_equation { out.push(Tok::Equals) },
            'x' => {
                let prev_num = matches!(out.last(), Some(Tok::Num(_)) | Some(Tok::RParen));
                let spaced_before = i > 0 && s[i - 1] == ' ';
                let next = s[i + 1..].iter().find(|c| **c != ' ');
                let next_num = next.map_or(false, |c| c.is_ascii_digit() || *c == '(');
                if !is_equation && prev_num && next_num || (is_equation && prev_num && spaced_before && next_num && s.get(i + 1) == Some(&' ')) {
                    out.push(Tok::Op('×'));
                } else {
                    out.push(Tok::X);
                }
            }
            _ => {}
        }
        i += 1;
    }
    // "x2" right after x usually means x² in printed text only when written as superscript; OCR gives "x2" or "x^2".
    // Treat X followed directly by the number 2 (no operator) as x².
    if out.len() > 2 && out[0] == Tok::Op('−') && !text.trim_start().starts_with("- ") { /* keep: real negative number */ }
    let mut fixed: Vec<Tok> = vec![];
    for t in out {
        if let (Some(Tok::X), Tok::Num(n)) = (fixed.last(), &t) { if n == "2" { fixed.push(Tok::Post("²")); continue; } }
        fixed.push(t);
    }
    fixed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{display_tokens, Evaluator};
    fn solve(s: &str) -> Vec<f64> { Evaluator::new(true).solve(&to_tokens(s)).unwrap() }
    fn calc(s: &str) -> f64 { Evaluator::new(true).eval(&to_tokens(s)).unwrap() }
    #[test] fn text_to_tokens() {
        assert_eq!(calc("12 + 7 ="), 19.0);
        assert_eq!(calc("3 x 4"), 12.0);
        assert_eq!(calc("12,5 : 5 ="), 2.5);
        assert_eq!(calc("(2+3)*4"), 20.0);
        assert_eq!(solve("2x + 3 = 11"), vec![4.0]);
        assert_eq!(solve("x2 - 5x + 6 = 0"), vec![2.0, 3.0]);
        assert_eq!(solve("x^2-5x+6=0"), vec![2.0, 3.0]);
        assert_eq!(solve("3(x-1) = x + 5"), vec![4.0]);
        assert_eq!(display_tokens(&to_tokens("2x + 3 = 11")), "2x + 3 = 11");
    }
}
