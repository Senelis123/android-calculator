//! Programmer mode: integer math in decimal, hex, binary and octal with bitwise operators.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BitOp { And, Or, Xor, Shl, Shr, Add, Sub, Mul, Div }

#[derive(Debug, Clone)]
pub struct Prog {
    pub text: String,   // current entry in `base`
    pub base: u8,       // 2, 8, 10 or 16
    pub acc: u64,
    pub op: Option<BitOp>,
    pub fresh: bool,    // next digit starts a new number
    pub error: bool,
}

impl Default for Prog {
    fn default() -> Self {
        Prog { text: "0".into(), base: 10, acc: 0, op: None, fresh: true, error: false }
    }
}

pub fn fmt(mut v: u64, base: u8) -> String {
    if v == 0 { return "0".into(); }
    let digits = b"0123456789ABCDEF";
    let mut out = vec![];
    while v > 0 {
        out.push(digits[(v % base as u64) as usize] as char);
        v /= base as u64;
    }
    out.iter().rev().collect()
}

pub fn parse(s: &str, base: u8) -> Option<u64> {
    if s.is_empty() { return None; }
    let mut v: u64 = 0;
    for c in s.chars() {
        let d = c.to_digit(16)? as u8;
        if d >= base { return None; }
        v = v.wrapping_mul(base as u64).wrapping_add(d as u64);
    }
    Some(v)
}

impl Prog {
    pub fn value(&self) -> u64 { parse(&self.text, self.base).unwrap_or(0) }

    pub fn set_base(&mut self, base: u8) {
        let v = self.value();
        self.base = base;
        self.text = fmt(v, base);
    }

    fn digit_limit(&self) -> usize {
        match self.base { 2 => 64, 8 => 22, 10 => 20, _ => 16 }
    }

    pub fn key(&mut self, k: &str) {
        match k {
            "AC" => { self.text = "0".into(); self.acc = 0; self.op = None; self.fresh = true; self.error = false; }
            // In hex mode "C" is a digit, so full clear lives on "AC" there.
            "C" if self.base != 16 => { self.text = "0".into(); self.acc = 0; self.op = None; self.fresh = true; self.error = false; }
            "CE" => { self.text = "0".into(); self.fresh = true; self.error = false; }
            "⌫" => {
                if self.fresh { return; }
                self.text.pop();
                if self.text.is_empty() { self.text = "0".into(); self.fresh = true; }
            }
            "AND" | "OR" | "XOR" | "<<" | ">>" | "+" | "−" | "×" | "÷" => {
                let op = match k {
                    "AND" => BitOp::And, "OR" => BitOp::Or, "XOR" => BitOp::Xor,
                    "<<" => BitOp::Shl, ">>" => BitOp::Shr, "+" => BitOp::Add,
                    "−" => BitOp::Sub, "×" => BitOp::Mul, _ => BitOp::Div,
                };
                if let Some(pending) = self.op.take() {
                    if !self.fresh { self.apply_pending(pending); }
                }
                self.acc = self.value();
                self.op = Some(op);
                self.fresh = true;
            }
            "=" => {
                if let Some(pending) = self.op.take() {
                    if !self.fresh { self.apply_pending(pending); }
                    self.fresh = true;
                }
            }
            "NOT" => {
                let v = !self.value();
                self.text = fmt(v, self.base);
                self.fresh = true;
            }
            d => {
                let c = match d { "A" | "B" | "C" | "D" | "E" | "F" => d.chars().next().unwrap(), _ => d.chars().next().unwrap_or('?') };
                let Some(dv) = c.to_digit(16) else { return; };
                if dv >= self.base as u32 { return; }
                if self.fresh {
                    self.text = c.to_string();
                    self.fresh = false;
                    self.error = false;
                } else if self.text == "0" {
                    self.text = c.to_string();
                } else if self.text.len() < self.digit_limit() {
                    self.text.push(c);
                }
            }
        }
    }

    fn apply_pending(&mut self, op: BitOp) {
        let (a, b) = (self.acc, self.value());
        let r = match op {
            BitOp::And => a & b,
            BitOp::Or => a | b,
            BitOp::Xor => a ^ b,
            BitOp::Shl => if b < 64 { a.wrapping_shl(b as u32) } else { 0 },
            BitOp::Shr => if b < 64 { a.wrapping_shr(b as u32) } else { 0 },
            BitOp::Add => a.wrapping_add(b),
            BitOp::Sub => a.wrapping_sub(b),
            BitOp::Mul => a.wrapping_mul(b),
            BitOp::Div => {
                if b == 0 {
                    self.error = true;
                    self.text = "0".into();
                    self.acc = 0;
                    return;
                }
                a / b
            }
        };
        self.acc = r;
        self.text = fmt(r, self.base);
    }

    /// (label, value) rows for the other bases plus signed decimal.
    pub fn conversions(&self) -> Vec<(String, String)> {
        let v = self.value();
        vec![
            ("DEC".into(), fmt(v, 10)),
            ("HEX".into(), fmt(v, 16)),
            ("BIN".into(), fmt(v, 2)),
            ("OCT".into(), fmt(v, 8)),
            ("±DEC".into(), (v as i64).to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bases_roundtrip() {
        assert_eq!(fmt(255, 16), "FF");
        assert_eq!(fmt(5, 2), "101");
        assert_eq!(parse("FF", 16), Some(255));
        assert_eq!(parse("101", 2), Some(5));
        assert_eq!(parse("8", 8), None);
    }
    #[test]
    fn bitwise() {
        let mut p = Prog::default();
        p.set_base(16);
        for k in ["F", "0"] { p.key(k); }
        p.key("AND");
        for k in ["3", "C"] { p.key(k); }
        p.key("=");
        assert_eq!(p.text, "30");
        p.key("AC");
        p.set_base(2);
        for k in ["1", "0", "1", "1"] { p.key(k); }
        p.key("<<");
        p.key("1");
        p.key("=");
        assert_eq!(p.text, "10110");
        p.key("NOT");
        assert_eq!(p.value(), !0b10110u64);
    }
    #[test]
    fn arithmetic_and_div_zero() {
        let mut p = Prog::default();
        for k in ["7", "×", "6", "="] { p.key(k); }
        assert_eq!(p.text, "42");
        for k in ["AC", "1", "÷", "0", "="] { p.key(k); }
        assert!(p.error);
    }
}
