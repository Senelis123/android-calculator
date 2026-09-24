//! Expression entry, evaluation (with operator precedence), number formatting and equation solving.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(String),        // raw digits with '.' decimal, e.g. "12.5"
    Op(char),           // + − × ÷ ^
    LParen,
    RParen,
    Func(&'static str), // sin cos tan ln log √  (acts as an opening bracket)
    Post(&'static str), // ² ! % ⁻¹
    Pi,
    E,
    X,
    Equals,             // equation sign (only when the expression contains x)
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError { DivZero, Domain, Syntax, NonPoly }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Poly(pub [f64; 3]); // c0 + c1 x + c2 x²

impl Poly {
    pub fn c(v: f64) -> Self { Poly([v, 0.0, 0.0]) }
    fn is_const(&self) -> bool { self.0[1] == 0.0 && self.0[2] == 0.0 }
    fn deg(&self) -> usize { if self.0[2] != 0.0 { 2 } else if self.0[1] != 0.0 { 1 } else { 0 } }
    fn konst(&self) -> Result<f64, CalcError> { if self.is_const() { Ok(self.0[0]) } else { Err(CalcError::NonPoly) } }
    fn add(self, o: Poly) -> Poly { Poly([self.0[0] + o.0[0], self.0[1] + o.0[1], self.0[2] + o.0[2]]) }
    fn neg(self) -> Poly { Poly([-self.0[0], -self.0[1], -self.0[2]]) }
    fn mul(self, o: Poly) -> Result<Poly, CalcError> {
        if self.deg() + o.deg() > 2 { return Err(CalcError::NonPoly); }
        let (a, b) = (self.0, o.0);
        Ok(Poly([a[0] * b[0], a[0] * b[1] + a[1] * b[0], a[0] * b[2] + a[1] * b[1] + a[2] * b[0]]))
    }
    fn div(self, o: Poly) -> Result<Poly, CalcError> {
        let d = o.konst()?;
        if d == 0.0 { return Err(CalcError::DivZero); }
        Ok(Poly([self.0[0] / d, self.0[1] / d, self.0[2] / d]))
    }
}

pub struct Evaluator { pub degrees: bool }

struct Parser<'a> { t: &'a [Tok], i: usize, deg: bool }

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> { self.t.get(self.i) }
    fn starts_factor(t: &Tok) -> bool { matches!(t, Tok::Num(_) | Tok::LParen | Tok::Func(_) | Tok::Pi | Tok::E | Tok::X) }

    fn expr(&mut self) -> Result<Poly, CalcError> {
        let (mut acc, _) = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '−'))) = self.peek().cloned() {
            self.i += 1;
            let (rhs, pct) = self.term()?;
            // a ± b%  ->  a ± a·b/100 (the usual calculator rule)
            let rhs = if pct { acc.mul(rhs)? } else { rhs };
            acc = if c == '+' { acc.add(rhs) } else { acc.add(rhs.neg()) };
        }
        Ok(acc)
    }

    /// Returns (value, is_single_percent_factor).
    fn term(&mut self) -> Result<(Poly, bool), CalcError> {
        let (mut acc, mut pct) = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op('×')) => { self.i += 1; acc = acc.mul(self.unary()?.0)?; pct = false; }
                Some(Tok::Op('÷')) => { self.i += 1; acc = acc.div(self.unary()?.0)?; pct = false; }
                Some(t) if Self::starts_factor(&t) => { acc = acc.mul(self.unary()?.0)?; pct = false; } // implicit ×
                _ => break,
            }
        }
        Ok((acc, pct))
    }

    fn unary(&mut self) -> Result<(Poly, bool), CalcError> {
        match self.peek() {
            Some(Tok::Op('−')) => { self.i += 1; let (v, p) = self.unary()?; Ok((v.neg(), p)) }
            Some(Tok::Op('+')) => { self.i += 1; self.unary() }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<(Poly, bool), CalcError> {
        let (base, pct) = self.postfix()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.i += 1;
            let (e, _) = self.unary()?;
            return Ok((pow(base, e.konst()?)?, false));
        }
        Ok((base, pct))
    }

    fn postfix(&mut self) -> Result<(Poly, bool), CalcError> {
        let mut v = self.primary()?;
        let mut pct = false;
        while let Some(Tok::Post(p)) = self.peek().cloned() {
            self.i += 1;
            pct = false;
            v = match p {
                "²" => v.mul(v)?,
                "%" => { pct = true; v.div(Poly::c(100.0))? }
                "⁻¹" => Poly::c(1.0).div(v)?,
                "!" => Poly::c(factorial(v.konst()?)?),
                _ => return Err(CalcError::Syntax),
            };
        }
        Ok((v, pct))
    }

    fn primary(&mut self) -> Result<Poly, CalcError> {
        let t = self.peek().cloned().ok_or(CalcError::Syntax)?;
        self.i += 1;
        match t {
            Tok::Num(s) => s.parse::<f64>().map(Poly::c).map_err(|_| CalcError::Syntax),
            Tok::Pi => Ok(Poly::c(std::f64::consts::PI)),
            Tok::E => Ok(Poly::c(std::f64::consts::E)),
            Tok::X => Ok(Poly([0.0, 1.0, 0.0])),
            Tok::LParen => { let v = self.expr()?; self.close(); Ok(v) }
            Tok::Func(f) => {
                let v = self.expr()?;
                self.close();
                if f == "√" {
                    let x = v.konst()?;
                    if x < 0.0 { return Err(CalcError::Domain); }
                    return Ok(Poly::c(x.sqrt()));
                }
                let x = v.konst()?;
                let r = |x: f64| if self.deg { x.to_radians() } else { x };
                let out = match f {
                    "sin" => snap(r(x).sin()),
                    "cos" => snap(r(x).cos()),
                    "tan" => {
                        if self.deg && (x / 90.0).fract() == 0.0 && (x / 90.0) as i64 % 2 != 0 { return Err(CalcError::Domain); }
                        snap(r(x).tan())
                    }
                    "ln" => { if x <= 0.0 { return Err(CalcError::Domain); } x.ln() }
                    "log" => { if x <= 0.0 { return Err(CalcError::Domain); } x.log10() }
                    _ => return Err(CalcError::Syntax),
                };
                Ok(Poly::c(out))
            }
            _ => Err(CalcError::Syntax),
        }
    }

    /// Closing bracket; missing ones at the end are closed automatically.
    fn close(&mut self) { if let Some(Tok::RParen) = self.peek() { self.i += 1; } }
}

fn snap(v: f64) -> f64 { if v.abs() < 1e-12 { 0.0 } else { v } }

fn pow(base: Poly, e: f64) -> Result<Poly, CalcError> {
    if base.is_const() {
        let b = base.0[0];
        if b == 0.0 && e < 0.0 { return Err(CalcError::DivZero); }
        let r = b.powf(e);
        if r.is_nan() { return Err(CalcError::Domain); }
        return Ok(Poly::c(r));
    }
    match e {
        e if e == 0.0 => Ok(Poly::c(1.0)),
        e if e == 1.0 => Ok(base),
        e if e == 2.0 => base.mul(base),
        _ => Err(CalcError::NonPoly),
    }
}

fn factorial(x: f64) -> Result<f64, CalcError> {
    if x < 0.0 || x.fract() != 0.0 || x > 170.0 { return Err(CalcError::Domain); }
    Ok((1..=x as u64).fold(1.0, |a, n| a * n as f64))
}

/// Drops trailing operators / open functions so a half-typed expression can still be previewed.
fn trimmed(t: &[Tok]) -> &[Tok] {
    let mut n = t.len();
    while n > 0 && matches!(t[n - 1], Tok::Op(_) | Tok::LParen | Tok::Func(_) | Tok::Equals) { n -= 1; }
    &t[..n]
}

impl Evaluator {
    pub fn eval_poly(&self, t: &[Tok]) -> Result<Poly, CalcError> {
        let t = trimmed(t);
        if t.is_empty() { return Ok(Poly::c(0.0)); }
        let mut p = Parser { t, i: 0, deg: self.degrees };
        let v = p.expr()?;
        // Stray closing brackets are ignored; anything else left over is a syntax error.
        while let Some(Tok::RParen) = p.peek() { p.i += 1; }
        if p.i != t.len() { return Err(CalcError::Syntax); }
        Ok(v)
    }

    pub fn eval(&self, t: &[Tok]) -> Result<f64, CalcError> {
        let v = self.eval_poly(t)?.konst()?;
        if v.is_nan() || v.is_infinite() { return Err(CalcError::Domain); }
        Ok(v)
    }

    /// Solves `left = right` (linear or quadratic in x). Returns the real roots.
    pub fn solve(&self, t: &[Tok]) -> Result<Vec<f64>, SolveError> {
        let pos = t.iter().position(|x| *x == Tok::Equals);
        let (l, r) = match pos { Some(p) => (&t[..p], &t[p + 1..]), None => (t, &[][..]) };
        let lp = self.eval_poly(l).map_err(SolveError::Calc)?;
        let rp = self.eval_poly(r).map_err(SolveError::Calc)?;
        let p = lp.add(rp.neg());
        let [c, b, a] = p.0.map(|v| if v.abs() < 1e-12 { 0.0 } else { v });
        if a == 0.0 {
            if b == 0.0 { return Err(if c == 0.0 { SolveError::AllX } else { SolveError::NoSolution }); }
            return Ok(vec![-c / b]);
        }
        let d = b * b - 4.0 * a * c;
        if d < -1e-12 { return Err(SolveError::NoRealSolution); }
        if d.abs() <= 1e-12 { return Ok(vec![-b / (2.0 * a)]); }
        let s = d.sqrt();
        let mut v = vec![(-b - s) / (2.0 * a), (-b + s) / (2.0 * a)];
        v.sort_by(|x, y| x.partial_cmp(y).unwrap());
        Ok(v)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SolveError { Calc(CalcError), NoSolution, AllX, NoRealSolution }

// ---------- number formatting (Lithuanian style: "1 234 567,89") ----------

pub const GROUP: char = '\u{202F}'; // narrow no-break space

fn group_int(int: &str) -> String {
    let (sign, digits) = match int.strip_prefix('-') { Some(d) => ("−", d), None => ("", int) };
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 { out.push(GROUP); }
        out.push(ch);
    }
    format!("{sign}{out}")
}

/// Formats a raw typed number ("1234.5", "12.") for display.
pub fn display_raw(raw: &str) -> String {
    match raw.split_once('.') {
        Some((i, f)) => format!("{},{}", group_int(if i.is_empty() { "0" } else { i }), f),
        None => group_int(raw),
    }
}

/// Plain machine form of a result (max 10 decimals, trailing zeros removed), e.g. "-1234.5".
pub fn plain(mut v: f64) -> String {
    if v.abs() < 1e-10 { v = 0.0; }
    if v.abs() >= 1e15 { return format!("{:e}", v); }
    let s = crate::calc::format(v); // shortest-digit rounding to 10 decimals (same rules as v1)
    if s == "-0" { "0".into() } else { s }
}

/// Display form of a result: grouped with comma decimals; huge numbers in E notation.
pub fn display_num(v: f64) -> String {
    let p = plain(v);
    if let Some((m, e)) = p.split_once('e') {
        let m: f64 = m.parse().unwrap_or(0.0);
        let m = format!("{:.9}", m);
        let m = m.trim_end_matches('0').trim_end_matches('.').replace('.', ",").replace('-', "−");
        return format!("{m}E{e}");
    }
    display_raw(&p)
}

/// Clipboard form: "1234,5" (no grouping, comma decimal).
pub fn clipboard_num(v: f64) -> String { plain(v).replace('.', ",") }

/// Parses pasted text like "1 234,50", "1,234.50" or "-12.5" into a raw number string.
pub fn parse_pasted(s: &str) -> Option<String> {
    let s: String = s.trim().chars().filter(|c| !c.is_whitespace() && *c != GROUP && *c != '\u{A0}').collect();
    let s = s.replace('−', "-");
    let s = if s.contains(',') && s.contains('.') {
        if s.rfind(',') > s.rfind('.') { s.replace('.', "").replace(',', ".") } else { s.replace(',', "") }
    } else { s.replace(',', ".") };
    let v: f64 = s.parse().ok()?;
    if !v.is_finite() { return None; }
    Some(if s.starts_with('.') { format!("0{s}") } else { s })
}

pub fn display_tokens(t: &[Tok]) -> String {
    let mut s = String::new();
    for tok in t {
        match tok {
            Tok::Num(n) => s.push_str(&display_raw(n)),
            Tok::Op('^') => s.push('^'),
            Tok::Op(c) => { s.push(' '); s.push(*c); s.push(' '); }
            Tok::LParen => s.push('('),
            Tok::RParen => s.push(')'),
            Tok::Func(f) => { s.push_str(f); s.push('('); }
            Tok::Post(p) => s.push_str(p),
            Tok::Pi => s.push('π'),
            Tok::E => s.push('e'),
            Tok::X => s.push('x'),
            Tok::Equals => s.push_str(" = "),
        }
    }
    s.replace("  ", " ").trim().to_string()
}

pub fn error_text(e: &CalcError) -> &'static str {
    match e { CalcError::DivZero => "Dalyba iš nulio", CalcError::Domain => "Klaida", CalcError::Syntax => "Klaida", CalcError::NonPoly => "Per sudėtinga" }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn toks(s: &str) -> Vec<Tok> {
        // tiny test tokenizer: numbers, operators, brackets, π e x, and words sin( cos( ...
        let mut v = vec![];
        let cs: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            let c = cs[i];
            if c.is_ascii_digit() || c == '.' {
                let st = i; while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') { i += 1; }
                v.push(Tok::Num(cs[st..i].iter().collect())); continue;
            }
            let rest: String = cs[i..].iter().collect();
            let mut matched = false;
            for f in ["sin", "cos", "tan", "ln", "log", "√"] {
                if rest.starts_with(f) { v.push(Tok::Func(f)); i += f.chars().count(); if cs.get(i) == Some(&'(') { i += 1; } matched = true; break; }
            }
            if matched { continue; }
            v.push(match c {
                '+' | '−' | '×' | '÷' | '^' => Tok::Op(c), '-' => Tok::Op('−'), '*' => Tok::Op('×'), '/' => Tok::Op('÷'),
                '(' => Tok::LParen, ')' => Tok::RParen, 'π' => Tok::Pi, 'e' => Tok::E, 'x' => Tok::X, '=' => Tok::Equals,
                '²' => Tok::Post("²"), '!' => Tok::Post("!"), '%' => Tok::Post("%"), '⁻' => { i += 1; Tok::Post("⁻¹") }
                _ => panic!("bad {c}"),
            });
            i += 1;
        }
        v
    }
    fn ev(s: &str) -> Result<f64, CalcError> { Evaluator { degrees: true }.eval(&toks(s)) }
    fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

    #[test] fn precedence() {
        assert_eq!(ev("2+3×4"), Ok(14.0));
        assert_eq!(ev("(2+3)×4"), Ok(20.0));
        assert_eq!(ev("2^3^2"), Ok(512.0));
        assert_eq!(ev("−2^2"), Ok(-4.0));
        assert_eq!(ev("10÷4"), Ok(2.5));
        assert_eq!(ev("2(3+4)"), Ok(14.0));
        assert!(close(ev("2π").unwrap(), 2.0 * std::f64::consts::PI));
        assert_eq!(ev("(1+2"), Ok(3.0)); // auto-close
        assert_eq!(ev("5+"), Ok(5.0));  // trailing operator ignored in preview
        assert_eq!(ev("1÷0"), Err(CalcError::DivZero));
    }
    #[test] fn functions() {
        assert_eq!(ev("sin(30)"), Ok(0.49999999999999994_f64.max(ev("sin(30)").unwrap())));
        assert!(close(ev("sin(30)").unwrap(), 0.5));
        assert_eq!(ev("cos(90)"), Ok(0.0));
        assert_eq!(ev("tan(90)"), Err(CalcError::Domain));
        assert_eq!(ev("√(16)"), Ok(4.0));
        assert_eq!(ev("√(16)+1"), Ok(5.0));
        assert_eq!(ev("√(−1)"), Err(CalcError::Domain));
        assert_eq!(ev("log(1000)"), Ok(3.0));
        assert_eq!(ev("5!"), Ok(120.0));
        assert_eq!(ev("3²"), Ok(9.0));
        assert_eq!(ev("4⁻¹"), Ok(0.25));
        assert!(close(ev("ln(e)").unwrap(), 1.0));
        assert!(close(Evaluator { degrees: false }.eval(&toks("sin(π÷2)")).unwrap(), 1.0));
    }
    #[test] fn percent() {
        assert_eq!(ev("50%"), Ok(0.5));
        assert_eq!(ev("200+10%"), Ok(220.0));
        assert_eq!(ev("200−10%"), Ok(180.0));
        assert_eq!(ev("200×10%"), Ok(20.0));
    }
    #[test] fn solving() {
        let s = |q: &str| Evaluator { degrees: true }.solve(&toks(q));
        assert_eq!(s("2x+3=11"), Ok(vec![4.0]));
        assert_eq!(s("3(x−1)=x+5"), Ok(vec![4.0]));
        assert_eq!(s("x÷2=7"), Ok(vec![14.0]));
        assert_eq!(s("x²−5x+6=0"), Ok(vec![2.0, 3.0]));
        assert_eq!(s("x^2=−1"), Err(SolveError::NoRealSolution));
        assert_eq!(s("x+1=x+2"), Err(SolveError::NoSolution));
        assert_eq!(s("2x=x+x"), Err(SolveError::AllX));
    }
    #[test] fn formatting() {
        assert_eq!(display_num(1234567.891), "1\u{202F}234\u{202F}567,891");
        assert_eq!(display_num(-1234.5), "−1\u{202F}234,5");
        assert_eq!(display_num(2.0 / 3.0), "0,6666666667");
        assert_eq!(display_num(0.1 + 0.2), "0,3");
        assert_eq!(display_num(123.0), "123");
        assert_eq!(display_num(1e20), "1E20");
        assert_eq!(display_raw("12."), "12,");
        assert_eq!(clipboard_num(-1234.5), "-1234,5");
        assert_eq!(parse_pasted("1 234,50"), Some("1234.50".into()));
        assert_eq!(parse_pasted("1,234.50"), Some("1234.50".into()));
        assert_eq!(parse_pasted("abc"), None);
        assert_eq!(display_tokens(&toks("2+sin(30)×x²")), "2 + sin(30) × x²");
    }
}
