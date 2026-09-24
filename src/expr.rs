//! Expression entry, evaluation (with operator precedence), number formatting and equation solving.

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(String),        // raw digits with '.' decimal, e.g. "12.5" or "1.2e5"
    Op(char),           // + − × ÷ ^ and 'm' (mod)
    LParen,
    RParen,
    Func(&'static str), // sin cos tan asin … ln log √ root logb nCr nPr abs (acts as an opening bracket)
    Post(&'static str), // ² ! % ⁻¹
    Sep,                // argument separator in root(n;x), logb(b;x), nCr(n;r)
    Pi,
    E,
    X,
    I,                  // imaginary unit (complex numbers)
    Ans,                // last answer
    Equals,             // equation sign (only when the expression contains x)
    Cmp(char),          // < > ≤ ≥ for inequalities
}

pub const FUNCS: &[&str] = &["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "ln", "log", "√", "root", "logb", "nCr", "nPr", "abs"];

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError { DivZero, Domain, Syntax, NonPoly }

const N: usize = 4;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Poly(pub [f64; N]); // c0 + c1 x + c2 x² + c3 x³

fn clean_sum(a: f64, b: f64) -> f64 {
    let s = a + b;
    // 0.1 + 0.2 − 0.3 style noise: cancel to exact zero relative to the operands
    if s != 0.0 && s.abs() < 1e-13 * a.abs().max(b.abs()) { 0.0 } else { s }
}

impl Poly {
    pub fn c(v: f64) -> Self { Poly([v, 0.0, 0.0, 0.0]) }
    fn x() -> Self { Poly([0.0, 1.0, 0.0, 0.0]) }
    fn is_const(&self) -> bool { self.0[1..].iter().all(|v| *v == 0.0) }
    fn deg(&self) -> usize { (0..N).rev().find(|&i| self.0[i] != 0.0).unwrap_or(0) }
    fn konst(&self) -> Result<f64, CalcError> { if self.is_const() { Ok(self.0[0]) } else { Err(CalcError::NonPoly) } }
    fn add(self, o: Poly) -> Poly { let mut r = [0.0; N]; for i in 0..N { r[i] = clean_sum(self.0[i], o.0[i]); } Poly(r) }
    fn neg(self) -> Poly { Poly(self.0.map(|v| -v)) }
    fn mul(self, o: Poly) -> Result<Poly, CalcError> {
        if self.deg() + o.deg() >= N { return Err(CalcError::NonPoly); }
        let mut r = [0.0; N];
        for i in 0..N { for j in 0..N - i { r[i + j] += self.0[i] * o.0[j]; } }
        Ok(Poly(r))
    }
    fn div(self, o: Poly) -> Result<Poly, CalcError> {
        let d = o.konst()?;
        if d == 0.0 { return Err(CalcError::DivZero); }
        Ok(Poly(self.0.map(|v| v / d)))
    }
    pub fn at(&self, x: f64) -> f64 { self.0.iter().rev().fold(0.0, |a, c| a * x + c) }
    /// Derivative coefficients.
    pub fn deriv(&self) -> Poly { let mut r = [0.0; N]; for i in 1..N { r[i - 1] = self.0[i] * i as f64; } Poly(r) }
}

#[derive(Debug, Clone, Copy)]
pub struct Evaluator { pub degrees: bool, pub ans: f64, pub xval: Option<f64> }
impl Evaluator { pub fn new(degrees: bool) -> Self { Evaluator { degrees, ans: 0.0, xval: None } } }

struct Parser<'a> { t: &'a [Tok], i: usize, ev: Evaluator }

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> { self.t.get(self.i) }
    fn starts_factor(t: &Tok) -> bool { matches!(t, Tok::Num(_) | Tok::LParen | Tok::Func(_) | Tok::Pi | Tok::E | Tok::X | Tok::Ans) }

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
                Some(Tok::Op('m')) => {
                    self.i += 1;
                    let (a, b) = (acc.konst()?, self.unary()?.0.konst()?);
                    if b == 0.0 { return Err(CalcError::DivZero); }
                    acc = Poly::c(a - b * (a / b).floor()); pct = false;
                }
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
            Tok::Ans => Ok(Poly::c(self.ev.ans)),
            Tok::X => Ok(match self.ev.xval { Some(v) => Poly::c(v), None => Poly::x() }),
            Tok::LParen => { let v = self.expr()?; self.close(); Ok(v) }
            Tok::Func(f) => {
                let v = self.expr()?;
                let second = if let Some(Tok::Sep) = self.peek() { self.i += 1; Some(self.expr()?.konst()?) } else { None };
                self.close();
                let x = v.konst()?;
                Ok(Poly::c(func(f, x, second, self.ev.degrees)?))
            }
            _ => Err(CalcError::Syntax),
        }
    }

    /// Closing bracket; missing ones at the end are closed automatically.
    fn close(&mut self) { if let Some(Tok::RParen) = self.peek() { self.i += 1; } }
}

fn func(f: &str, x: f64, y: Option<f64>, deg: bool) -> Result<f64, CalcError> {
    let r = |x: f64| if deg { x.to_radians() } else { x };
    let back = |v: f64| if deg { v.to_degrees() } else { v };
    let dom = |ok: bool| if ok { Ok(()) } else { Err(CalcError::Domain) };
    let two = || y.ok_or(CalcError::Syntax);
    let out = match f {
        "√" => { dom(x >= 0.0)?; x.sqrt() }
        "sin" => snap(r(x).sin()),
        "cos" => snap(r(x).cos()),
        "tan" => {
            if deg && (x / 90.0).fract() == 0.0 && (x / 90.0) as i64 % 2 != 0 { return Err(CalcError::Domain); }
            snap(r(x).tan())
        }
        "asin" => { dom(x.abs() <= 1.0)?; back(x.asin()) }
        "acos" => { dom(x.abs() <= 1.0)?; back(x.acos()) }
        "atan" => back(x.atan()),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "abs" => x.abs(),
        "ln" => { dom(x > 0.0)?; x.ln() }
        "log" => { dom(x > 0.0)?; x.log10() }
        // root(n; x): n-th root of x (odd roots of negatives allowed)
        "root" => {
            let (n, v) = (x, two()?);
            dom(n != 0.0)?;
            if v < 0.0 { dom(n.fract() == 0.0 && (n as i64) % 2 != 0)?; -(-v).powf(1.0 / n) } else { snap_int(v.powf(1.0 / n)) }
        }
        // logb(b; x): logarithm of x with base b
        "logb" => { let v = two()?; dom(x > 0.0 && x != 1.0 && v > 0.0)?; snap_int(v.ln() / x.ln()) }
        "nCr" | "nPr" => {
            let k = two()?;
            dom(x >= 0.0 && k >= 0.0 && x.fract() == 0.0 && k.fract() == 0.0 && k <= x)?;
            let mut acc = 1.0;
            for i in 0..k as u64 { acc *= (x - i as f64) as f64; if f == "nCr" { acc /= (i + 1) as f64; } }
            acc.round()
        }
        _ => return Err(CalcError::Syntax),
    };
    if out.is_nan() { return Err(CalcError::Domain); }
    Ok(out)
}

fn snap(v: f64) -> f64 { if v.abs() < 1e-12 { 0.0 } else { v } }
fn snap_int(v: f64) -> f64 { if (v - v.round()).abs() < 1e-9 * v.abs().max(1.0) { v.round() } else { v } }

fn pow(base: Poly, e: f64) -> Result<Poly, CalcError> {
    if base.is_const() {
        let b = base.0[0];
        if b == 0.0 && e < 0.0 { return Err(CalcError::DivZero); }
        let r = b.powf(e);
        if r.is_nan() { return Err(CalcError::Domain); }
        return Ok(Poly::c(r));
    }
    if e >= 0.0 && e.fract() == 0.0 && e < N as f64 {
        let mut acc = Poly::c(1.0);
        for _ in 0..e as usize { acc = acc.mul(base)?; }
        return Ok(acc);
    }
    Err(CalcError::NonPoly)
}

fn factorial(x: f64) -> Result<f64, CalcError> {
    if x < 0.0 || x.fract() != 0.0 || x > 170.0 { return Err(CalcError::Domain); }
    Ok((1..=x as u64).fold(1.0, |a, n| a * n as f64))
}

/// Drops trailing operators / open functions so a half-typed expression can still be previewed.
fn trimmed(t: &[Tok]) -> &[Tok] {
    let mut n = t.len();
    while n > 0 && matches!(t[n - 1], Tok::Op(_) | Tok::LParen | Tok::Func(_) | Tok::Equals | Tok::Sep | Tok::Cmp(_)) { n -= 1; }
    &t[..n]
}

/// Result of solving an equation or inequality.
#[derive(Debug, Clone, PartialEq)]
pub enum Solution { Roots(Vec<f64>), Text(String) }

impl Evaluator {
    pub fn eval_poly(&self, t: &[Tok]) -> Result<Poly, CalcError> {
        let t = trimmed(t);
        if t.is_empty() { return Ok(Poly::c(0.0)); }
        let mut p = Parser { t, i: 0, ev: *self };
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

    /// f(x) for any expression in x (used by graphs, numeric solving and integrals).
    pub fn f_at(&self, t: &[Tok], x: f64) -> Option<f64> {
        let e = Evaluator { xval: Some(x), ..*self };
        e.eval(t).ok()
    }

    fn sides<'b>(&self, t: &'b [Tok]) -> (&'b [Tok], &'b [Tok], Option<Tok>) {
        match t.iter().position(|x| matches!(x, Tok::Equals | Tok::Cmp(_))) {
            Some(p) => (&t[..p], &t[p + 1..], Some(t[p].clone())),
            None => (t, &[][..], None),
        }
    }

    /// left − right as a polynomial (Err(NonPoly) when not polynomial up to x³).
    pub fn difference(&self, t: &[Tok]) -> Result<Poly, CalcError> {
        let (l, r, _) = self.sides(t);
        Ok(self.eval_poly(l)?.add(self.eval_poly(r)?.neg()))
    }

    /// Solves `left = right`: exactly up to cubic, numerically otherwise. Returns sorted real roots.
    pub fn solve(&self, t: &[Tok]) -> Result<Vec<f64>, SolveError> {
        match self.difference(t) {
            Ok(p) => poly_roots(&p),
            Err(CalcError::NonPoly) => {
                let (l, r, _) = self.sides(t);
                let f = |x: f64| Some(self.f_at(l, x)? - self.f_at(r, x).unwrap_or(0.0));
                let roots = numeric_roots(&f, -1000.0, 1000.0);
                if roots.is_empty() { Err(SolveError::NoRealSolution) } else { Ok(roots) }
            }
            Err(e) => Err(SolveError::Calc(e)),
        }
    }

    /// Solves an inequality like 2x + 3 > 7; returns the answer as text (x > 2, 1 < x < 3, ...).
    pub fn solve_ineq(&self, t: &[Tok]) -> Result<String, SolveError> {
        let (l, r, op) = self.sides(t);
        let op = match op { Some(Tok::Cmp(c)) => c, _ => return Err(SolveError::Calc(CalcError::Syntax)) };
        let f = |x: f64| -> Option<f64> { Some(self.f_at(l, x)? - self.f_at(r, x).unwrap_or(0.0)) };
        let roots = match self.difference(t) { Ok(p) => poly_roots(&p).unwrap_or_default(), Err(CalcError::NonPoly) => numeric_roots(&f, -1000.0, 1000.0), Err(e) => return Err(SolveError::Calc(e)) };
        let strict = op == '<' || op == '>';
        let holds = |x: f64| f(x).map(|v| match op { '<' => v < 0.0, '>' => v > 0.0, '≤' => v <= 1e-12, _ => v >= -1e-12 }).unwrap_or(false);
        // test each interval between roots
        let mut pts = vec![f64::NEG_INFINITY]; pts.extend(roots.iter().cloned()); pts.push(f64::INFINITY);
        let mut parts: Vec<String> = vec![];
        let (lt, gt) = if strict { ("<", ">") } else { ("≤", "≥") };
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let mid = if a.is_infinite() && b.is_infinite() { 0.0 } else if a.is_infinite() { b - 1.0 } else if b.is_infinite() { a + 1.0 } else { (a + b) / 2.0 };
            if !holds(mid) { continue; }
            parts.push(match (a.is_infinite(), b.is_infinite()) {
                (true, true) => crate::i18n::t("Any x works").to_string(),
                (true, false) => format!("x {lt} {}", display_num(b)),
                (false, true) => format!("x {gt} {}", display_num(a)),
                _ => format!("{} {lt} x {lt} {}", display_num(a), display_num(b)),
            });
        }
        // merge touching intervals for non-strict inequalities is left as separate parts
        if parts.is_empty() {
            // only the roots themselves may satisfy a non-strict inequality
            let pts: Vec<String> = roots.iter().filter(|x| holds(**x)).map(|x| format!("x = {}", display_num(*x))).collect();
            if pts.is_empty() { return Err(SolveError::NoSolution); }
            return Ok(pts.join("; "));
        }
        Ok(parts.join(&format!(" {} ", crate::i18n::t("or"))))
    }
}

/// Real roots of a polynomial up to degree 3, sorted.
pub fn poly_roots(p: &Poly) -> Result<Vec<f64>, SolveError> {
    let c = p.0.map(|v| if v.abs() < 1e-12 { 0.0 } else { v });
    let mut v = match p.deg() {
        0 => return Err(if c[0] == 0.0 { SolveError::AllX } else { SolveError::NoSolution }),
        1 => vec![-c[0] / c[1]],
        2 => {
            let (a, b, cc) = (c[2], c[1], c[0]);
            let d = b * b - 4.0 * a * cc;
            let scale = (b * b).abs().max((4.0 * a * cc).abs()).max(1e-300);
            if d < -1e-12 * scale { return Err(SolveError::NoRealSolution); }
            if d.abs() <= 1e-12 * scale { vec![-b / (2.0 * a)] } else { let s = d.sqrt(); vec![(-b - s) / (2.0 * a), (-b + s) / (2.0 * a)] }
        }
        _ => cubic(c[3], c[2], c[1], c[0]),
    };
    v = v.into_iter().map(|r| polish(p, r)).map(snap_int).map(|r| if r.abs() < 1e-12 { 0.0 } else { r }).collect();
    v.sort_by(|x, y| x.partial_cmp(y).unwrap());
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    Ok(v)
}

fn polish(p: &Poly, mut x: f64) -> f64 {
    let d = p.deriv();
    for _ in 0..8 { let dv = d.at(x); if dv.abs() < 1e-14 { break; } let nx = x - p.at(x) / dv; if !nx.is_finite() { break; } x = nx; }
    x
}

fn cubic(a: f64, b: f64, c: f64, d: f64) -> Vec<f64> {
    // depressed cubic t³ + pt + q with x = t − b/3a
    let (b, c, d) = (b / a, c / a, d / a);
    let p = c - b * b / 3.0;
    let q = 2.0 * b * b * b / 27.0 - b * c / 3.0 + d;
    let sh = -b / 3.0;
    let disc = q * q / 4.0 + p * p * p / 27.0;
    if disc.abs() < 1e-14 {
        if p.abs() < 1e-14 { return vec![sh]; }
        let u = (-q / 2.0).cbrt();
        return vec![2.0 * u + sh, -u + sh];
    }
    if disc > 0.0 {
        let s = disc.sqrt();
        vec![(-q / 2.0 + s).cbrt() + (-q / 2.0 - s).cbrt() + sh]
    } else {
        let r = (-p / 3.0).sqrt();
        let phi = (3.0 * q / (2.0 * p) * (-3.0 / p).sqrt()).clamp(-1.0, 1.0).acos();
        (0..3).map(|k| 2.0 * r * ((phi - 2.0 * std::f64::consts::PI * k as f64) / 3.0).cos() + sh).collect()
    }
}

/// Sign-change scan + bisection for any function (also finds touching roots where |f| is tiny).
pub fn numeric_roots(f: &dyn Fn(f64) -> Option<f64>, a: f64, b: f64) -> Vec<f64> {
    let steps = 20000;
    let h = (b - a) / steps as f64;
    let mut out: Vec<f64> = vec![];
    let mut prev: Option<(f64, f64)> = None;
    for i in 0..=steps {
        let x = a + i as f64 * h;
        let y = match f(x) { Some(y) if y.is_finite() => y, _ => { prev = None; continue; } };
        if y == 0.0 { out.push(x); }
        else if let Some((px, py)) = prev {
            if py != 0.0 && py.signum() != y.signum() {
                let (mut lo, mut hi, mut flo) = (px, x, py);
                for _ in 0..80 { let m = (lo + hi) / 2.0; let fm = match f(m) { Some(v) => v, None => break }; if fm.signum() == flo.signum() { lo = m; flo = fm; } else { hi = m; } }
                let r = (lo + hi) / 2.0;
                // skip poles (tan(x) jumps): value must be small near the root
                if f(r).map(|v| v.abs() < 1e-6).unwrap_or(false) { out.push(snap_int(r)); }
            }
        }
        prev = Some((x, y));
        if out.len() > 20 { break; }
    }
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    out
}

/// Simpson integral of f over [a, b].
pub fn integrate(f: &dyn Fn(f64) -> Option<f64>, a: f64, b: f64) -> Option<f64> {
    let n = 2000;
    let h = (b - a) / n as f64;
    let mut s = f(a)? + f(b)?;
    for i in 1..n { s += f(a + i as f64 * h)? * if i % 2 == 1 { 4.0 } else { 2.0 }; }
    Some(snap_int(s * h / 3.0))
}

/// Closest fraction p/q (q ≤ 10000) that equals v to ~1e-10, via continued fractions.
pub fn to_fraction(v: f64) -> Option<(i64, i64)> {
    if !v.is_finite() || v.abs() > 1e12 { return None; }
    let (mut h0, mut h1, mut k0, mut k1) = (0i64, 1i64, 1i64, 0i64);
    let mut x = v.abs();
    for _ in 0..40 {
        let a = x.floor();
        if a > 1e12 { break; }
        let a = a as i64;
        let (h2, k2) = (a.checked_mul(h1)?.checked_add(h0)?, a.checked_mul(k1)?.checked_add(k0)?);
        if k2 > 10000 { break; }
        h0 = h1; h1 = h2; k0 = k1; k1 = k2;
        if ((h1 as f64 / k1 as f64) - v.abs()).abs() < 1e-10 * v.abs().max(1.0) { return Some((if v < 0.0 { -h1 } else { h1 }, k1)); }
        let f = x - a as f64;
        if f < 1e-15 { break; }
        x = 1.0 / f;
    }
    None
}

pub fn fraction_text(v: f64) -> Option<String> {
    let (p, q) = to_fraction(v)?;
    if q == 1 { return None; }
    let sign = if p < 0 { "−" } else { "" };
    let p = p.abs();
    Some(if p > q { format!("{sign}{} {}/{}", p / q, p % q, q) } else { format!("{sign}{p}/{q}") })
}

pub fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
pub fn lcm(a: u64, b: u64) -> Option<u64> { if a == 0 || b == 0 { Some(0) } else { (a / gcd(a, b)).checked_mul(b) } }
pub fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut out = vec![];
    let mut p = 2u64;
    while p.saturating_mul(p) <= n {
        let mut e = 0; while n % p == 0 { n /= p; e += 1; }
        if e > 0 { out.push((p, e)); }
        p += if p == 2 { 1 } else { 2 };
    }
    if n > 1 { out.push((n, 1)); }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub enum SolveError { Calc(CalcError), NoSolution, AllX, NoRealSolution }

// ---------- number formatting (default Lithuanian style: "1 234 567,89") ----------

use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

pub const GROUP: char = '\u{202F}'; // narrow no-break space
static DEC_CH: AtomicU32 = AtomicU32::new(',' as u32);
static GROUP_CH: AtomicU32 = AtomicU32::new(GROUP as u32); // 0 = no grouping
static FIXED: AtomicI32 = AtomicI32::new(-1); // -1 = automatic decimals

/// Number format settings: decimal 0 = comma, 1 = dot; grouping 0 = space, 1 = none, 2 = dot/comma (opposite of decimal), 3 = apostrophe.
pub fn set_format(decimal: u8, grouping: u8, fixed: i32) {
    let dec = if decimal == 1 { '.' } else { ',' };
    let grp = match grouping { 1 => 0, 2 => if dec == ',' { '.' as u32 } else { ',' as u32 }, 3 => '\'' as u32, _ => GROUP as u32 };
    DEC_CH.store(dec as u32, Ordering::Relaxed);
    GROUP_CH.store(grp, Ordering::Relaxed);
    FIXED.store(fixed.clamp(-1, 10), Ordering::Relaxed);
}
pub fn dec_char() -> char { char::from_u32(DEC_CH.load(Ordering::Relaxed)).unwrap_or(',') }
fn group_char() -> Option<char> { char::from_u32(GROUP_CH.load(Ordering::Relaxed)).filter(|c| *c != '\0') }

fn group_int(int: &str) -> String {
    let (sign, digits) = match int.strip_prefix('-') { Some(d) => ("−", d), None => ("", int) };
    let g = group_char();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if let Some(g) = g { if i > 0 && (digits.len() - i) % 3 == 0 { out.push(g); } }
        out.push(ch);
    }
    format!("{sign}{out}")
}

/// Formats a raw typed number ("1234.5", "12.", "1.2e5") for display.
pub fn display_raw(raw: &str) -> String {
    if let Some((m, e)) = raw.split_once('e') { return format!("{}×10^{}", display_raw(m), e.replace('-', "−")); }
    match raw.split_once('.') {
        Some((i, f)) => format!("{}{}{}", group_int(if i.is_empty() { "0" } else { i }), dec_char(), f),
        None => group_int(raw),
    }
}

/// Plain machine form of a result (max 10 decimals, trailing zeros removed), e.g. "-1234.5".
/// Very large or very small values use exponent form "1.5e-12".
pub fn plain(v: f64) -> String {
    if v == 0.0 { return "0".into(); }
    if v.abs() >= 1e15 || v.abs() < 1e-9 {
        let s = format!("{:.12e}", v);
        let (m, e) = s.split_once('e').unwrap();
        let m = m.trim_end_matches('0').trim_end_matches('.');
        return format!("{m}e{e}");
    }
    let s = crate::calc::format(v); // shortest-digit rounding to 10 decimals (same rules as v1)
    if s == "-0" { "0".into() } else { s }
}

/// Display form of a result: grouped, with the chosen decimal sign; huge/tiny numbers in E notation.
pub fn display_num(v: f64) -> String {
    let fixed = FIXED.load(Ordering::Relaxed);
    let p = plain(v);
    if let Some((m, e)) = p.split_once('e') {
        let m: f64 = m.parse().unwrap_or(0.0);
        let m = format!("{:.9}", m);
        let m = m.trim_end_matches('0').trim_end_matches('.').replace('.', &dec_char().to_string()).replace('-', "−");
        return format!("{m}E{}", e.replace('-', "−"));
    }
    if fixed >= 0 {
        let s = format!("{:.*}", fixed as usize, v);
        let s = if s.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') { s.trim_start_matches('-').to_string() } else { s };
        return display_raw(&s);
    }
    display_raw(&p)
}

/// Clipboard form: "1234,5" (no grouping, chosen decimal sign).
pub fn clipboard_num(v: f64) -> String { plain(v).replace('.', &dec_char().to_string()) }

/// Parses pasted text like "1 234,50", "1,234.50", "1.234,50 €", "-12.5" or "1,5e3" into a raw number string.
pub fn parse_pasted(s: &str) -> Option<String> {
    let s: String = s.trim().chars().filter(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '−' | 'e' | 'E' | '+' | '\'')).collect();
    let s = s.replace('−', "-").replace('\'', "").replace('E', "e");
    let s = s.trim_start_matches('+').to_string();
    let s = if s.contains(',') && s.contains('.') {
        if s.rfind(',') > s.rfind('.') { s.replace('.', "").replace(',', ".") } else { s.replace(',', "") }
    } else if s.matches(',').count() > 1 { s.replace(',', "") }
    else if s.matches('.').count() > 1 { s.replace('.', "") }
    else { s.replace(',', ".") };
    let v: f64 = s.parse().ok()?;
    if !v.is_finite() { return None; }
    Some(if s.starts_with('.') { format!("0{s}") } else if s.starts_with("-.") { format!("-0{}", &s[1..]) } else { s })
}

pub fn display_tokens(t: &[Tok]) -> String {
    let mut s = String::new();
    for tok in t {
        match tok {
            Tok::Num(n) => s.push_str(&display_raw(n)),
            Tok::Op('^') => s.push('^'),
            Tok::Op('m') => s.push_str(" mod "),
            Tok::Op(c) => { s.push(' '); s.push(*c); s.push(' '); }
            Tok::LParen => s.push('('),
            Tok::RParen => s.push(')'),
            Tok::Func(f) => { s.push_str(match *f { "asin" => "sin⁻¹", "acos" => "cos⁻¹", "atan" => "tan⁻¹", "root" => "ⁿ√", f => f }); s.push('('); }
            Tok::Post(p) => s.push_str(p),
            Tok::Sep => s.push_str("; "),
            Tok::Pi => s.push('π'),
            Tok::E => s.push('e'),
            Tok::X => s.push('x'),
            Tok::I => s.push('i'),
            Tok::Ans => s.push_str("Ans"),
            Tok::Equals => s.push_str(" = "),
            Tok::Cmp(c) => { s.push(' '); s.push(*c); s.push(' '); }
        }
    }
    s.replace("  ", " ").trim().to_string()
}

pub fn error_text(e: &CalcError) -> &'static str {
    match e { CalcError::DivZero => crate::i18n::t("Division by zero"), CalcError::Domain => crate::i18n::t("Error"), CalcError::Syntax => crate::i18n::t("Error"), CalcError::NonPoly => crate::i18n::t("Too complex") }
}

// ---------- numbers in words ----------

fn en_words(n: u64) -> String {
    const ONES: [&str; 20] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"];
    const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    fn below_1000(n: u64) -> String {
        let mut p = vec![];
        if n >= 100 { p.push(format!("{} hundred", ONES[(n / 100) as usize])); }
        let r = n % 100;
        if r >= 20 { p.push(if r % 10 > 0 { format!("{}-{}", TENS[(r / 10) as usize], ONES[(r % 10) as usize]) } else { TENS[(r / 10) as usize].to_string() }); }
        else if r > 0 || n == 0 { p.push(ONES[r as usize].to_string()); }
        p.join(" ")
    }
    if n == 0 { return "zero".into(); }
    let scales = ["", " thousand", " million", " billion", " trillion", " quadrillion"];
    let mut parts = vec![]; let mut n = n; let mut i = 0;
    while n > 0 { let c = n % 1000; if c > 0 { parts.push(format!("{}{}", below_1000(c), scales[i])); } n /= 1000; i += 1; }
    parts.reverse(); parts.join(" ")
}

fn lt_words(n: u64) -> String {
    const ONES: [&str; 20] = ["nulis", "vienas", "du", "trys", "keturi", "penki", "šeši", "septyni", "aštuoni", "devyni", "dešimt", "vienuolika", "dvylika", "trylika", "keturiolika", "penkiolika", "šešiolika", "septyniolika", "aštuoniolika", "devyniolika"];
    const TENS: [&str; 10] = ["", "", "dvidešimt", "trisdešimt", "keturiasdešimt", "penkiasdešimt", "šešiasdešimt", "septyniasdešimt", "aštuoniasdešimt", "devyniasdešimt"];
    fn below_1000(n: u64) -> String {
        let mut p = vec![];
        let h = n / 100;
        if h == 1 { p.push("vienas šimtas".to_string()); } else if h > 1 { p.push(format!("{} šimtai", ONES[h as usize])); }
        let r = n % 100;
        if r >= 20 { p.push(TENS[(r / 10) as usize].to_string()); if r % 10 > 0 { p.push(ONES[(r % 10) as usize].to_string()); } }
        else if r > 0 { p.push(ONES[r as usize].to_string()); }
        p.join(" ")
    }
    // (singular, plural 2-9, genitive plural 10+ / teens / round tens)
    fn form(c: u64, f: [&str; 3]) -> &str { let (u, t) = (c % 10, c % 100); if u == 0 || (11..=19).contains(&t) { f[2] } else if u == 1 { f[0] } else { f[1] } }
    if n == 0 { return "nulis".into(); }
    let scales: [[&str; 3]; 5] = [["", "", ""], ["tūkstantis", "tūkstančiai", "tūkstančių"], ["milijonas", "milijonai", "milijonų"], ["milijardas", "milijardai", "milijardų"], ["trilijonas", "trilijonai", "trilijonų"]];
    let mut parts = vec![]; let mut n = n; let mut i = 0;
    while n > 0 && i < scales.len() {
        let c = n % 1000;
        if c > 0 { parts.push(if i == 0 { below_1000(c) } else { format!("{} {}", below_1000(c), form(c, scales[i])) }); }
        n /= 1000; i += 1;
    }
    parts.reverse(); parts.join(" ")
}

/// The number in words (Lithuanian for lang 1, English otherwise). None when too large.
pub fn words(v: f64, lang: usize) -> Option<String> {
    if !v.is_finite() || v.abs() >= 1e15 { return None; }
    let p = plain(v.abs());
    if p.contains('e') { return None; }
    let (int, frac) = match p.split_once('.') { Some((i, f)) => (i.to_string(), Some(f.to_string())), None => (p.clone(), None) };
    let n: u64 = int.parse().ok()?;
    let lt = lang == 1;
    let mut s = if lt { lt_words(n) } else { en_words(n) };
    if v < 0.0 { s = format!("{} {s}", if lt { "minus" } else { "minus" }); }
    if let Some(f) = frac {
        let digit = |d: char| { let d = d.to_digit(10).unwrap() as u64; if lt { lt_words(d) } else { en_words(d) } };
        s = format!("{s} {} {}", if lt { "kablelis" } else { "point" }, f.chars().map(digit).collect::<Vec<_>>().join(" "));
    }
    Some(s)
}

/// Turns plain text like "2*(3+4)^2", "sqrt(16)" or "15% of 200" style expressions into tokens.
/// Used for results from voice input and Gemini Nano. None when something is not understood.
pub fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let s = s.replace("**", "^").replace("sqrt", "√").replace('·', "×").replace('⋅', "×").replace('–', "-").replace(':', "÷");
    let mut v = vec![];
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() { i += 1; continue; }
        if c.is_ascii_digit() || c == '.' || (c == ',' && cs.get(i + 1).map_or(false, |d| d.is_ascii_digit()) && i > 0 && cs[i - 1].is_ascii_digit()) {
            let st = i; while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.' || (cs[i] == ',' && cs.get(i + 1).map_or(false, |d| d.is_ascii_digit()))) { i += 1; }
            let raw: String = cs[st..i].iter().collect::<String>().replace(',', ".");
            if raw.matches('.').count() > 1 || raw == "." { return None; }
            v.push(Tok::Num(raw)); continue;
        }
        let rest: String = cs[i..].iter().collect();
        let mut matched = false;
        for f in ["asin", "acos", "atan", "sinh", "cosh", "tanh", "sin", "cos", "tan", "ln", "log", "√", "abs"] {
            if rest.starts_with(f) { v.push(Tok::Func(f)); i += f.chars().count(); while cs.get(i).map_or(false, |c| c.is_whitespace()) { i += 1; } if cs.get(i) == Some(&'(') { i += 1; } matched = true; break; }
        }
        if matched { continue; }
        if rest.starts_with("pi") { v.push(Tok::Pi); i += 2; continue; }
        v.push(match c {
            '+' | '−' | '×' | '÷' | '^' => Tok::Op(c), '-' => Tok::Op('−'), '*' | 'x' | 'X' => Tok::Op('×'), '/' => Tok::Op('÷'),
            '(' | '[' => Tok::LParen, ')' | ']' => Tok::RParen, 'π' => Tok::Pi,
            '²' => Tok::Post("²"), '!' => Tok::Post("!"), '%' => Tok::Post("%"),
            _ => return None,
        });
        i += 1;
    }
    if v.is_empty() { None } else { Some(v) }
}

/// Evaluates text with the standard evaluator (degrees).
pub fn eval_text(s: &str) -> Option<f64> { let t = tokenize(s)?; Evaluator::new(true).eval(&t).ok() }

/// Offline reading of a spoken or typed problem ("du plius du", "15 procentų nuo 200", "what is 3 times 4").
/// Number words (EN/LT, 0-999 999) become digits, operator words become symbols. Returns an expression string.
pub fn text_to_expr(text: &str) -> Option<String> {
    let low = text.to_lowercase().replace(['?', '!', '„', '“', '"'], " ");
    // multi-word operators first
    let mut s = format!(" {} ", low);
    for (a, b) in [("divided by", "/"), ("padalinti iš", "/"), ("padalinta iš", "/"), ("dalinti iš", "/"), ("to the power of", "^"), ("kvadratu", "^2"), ("squared", "^2"), ("square root of", "sqrt "), ("kvadratinė šaknis iš", "sqrt "), ("šaknis iš", "sqrt "),
        ("procentų nuo", "% *"), ("procentai nuo", "% *"), ("procentas nuo", "% *"), ("percent of", "% *"), ("% of", "% *"), ("% nuo", "% *"), ("multiplied by", "*"), ("padauginti iš", "*"), ("padauginta iš", "*"), ("kablelis", "."), ("point", ".")] {
        s = s.replace(a, &format!(" {b} "));
    }
    let ones_en = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"];
    let tens_en = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    let ones_lt = ["nulis", "vienas", "du", "trys", "keturi", "penki", "šeši", "septyni", "aštuoni", "devyni", "dešimt", "vienuolika", "dvylika", "trylika", "keturiolika", "penkiolika", "šešiolika", "septyniolika", "aštuoniolika", "devyniolika"];
    let tens_lt = ["", "", "dvidešimt", "trisdešimt", "keturiasdešimt", "penkiasdešimt", "šešiasdešimt", "septyniasdešimt", "aštuoniasdešimt", "devyniasdešimt"];
    let word_num = |w: &str| -> Option<(u64, u8)> { // (value, kind) kind 0 = unit, 1 = hundred, 2 = thousand
        if let Some(i) = ones_en.iter().position(|x| *x == w).or_else(|| ones_lt.iter().position(|x| *x == w)) { return Some((i as u64, 0)); }
        if matches!(w, "viena" | "vieną") { return Some((1, 0)); }
        if matches!(w, "dvi") { return Some((2, 0)); }
        if let Some(i) = tens_en.iter().position(|x| !x.is_empty() && *x == w).or_else(|| tens_lt.iter().position(|x| !x.is_empty() && *x == w)) { return Some((i as u64 * 10, 0)); }
        if matches!(w, "hundred" | "šimtas" | "šimtai" | "šimtų") { return Some((100, 1)); }
        if matches!(w, "thousand" | "tūkstantis" | "tūkstančiai" | "tūkstančių") { return Some((1000, 2)); }
        None
    };
    let mut out: Vec<String> = vec![];
    let (mut cur, mut total, mut innum) = (0u64, 0u64, false);
    let flush = |out: &mut Vec<String>, cur: &mut u64, total: &mut u64, innum: &mut bool| { if *innum { out.push((*total + *cur).to_string()); } *cur = 0; *total = 0; *innum = false; };
    for w in s.split_whitespace() {
        let w = w.trim_matches(|c: char| c == '.' && false);
        if let Some((n, k)) = word_num(w) {
            match k { 0 => cur += n, 1 => cur = cur.max(1) * 100, _ => { total += cur.max(1) * 1000; cur = 0; } }
            innum = true; continue;
        }
        if w == "and" && innum { continue; }
        flush(&mut out, &mut cur, &mut total, &mut innum);
        let op = match w {
            "plus" | "plius" | "pridėti" | "sudėti" | "add" | "+" => "+",
            "minus" | "atėmus" | "atimti" | "take" | "−" | "-" => "-",
            "times" | "kart" | "kartų" | "padauginti" | "×" | "*" | "x" => "*",
            "dalinti" | "padalinti" | "over" | "÷" | "/" | ":" => "/",
            "procentų" | "procentai" | "procentas" | "percent" | "%" => "%",
            "laipsniu" | "power" | "^" => "^",
            "(" | "skliaustai" | "open" => "(", ")" | "close" => ")",
            _ => "",
        };
        if !op.is_empty() { out.push(op.to_string()); continue; }
        // numbers and symbols written inside the text, e.g. "15%" or "2+2"
        let keep: String = w.chars().filter(|c| c.is_ascii_digit() || "+-*/^()%.,√".contains(*c)).collect();
        let is_math = !keep.is_empty() && keep.len() * 2 >= w.chars().count();
        if is_math || w == "sqrt" { out.push(if w == "sqrt" { "sqrt".into() } else { keep }); }
    }
    flush(&mut out, &mut cur, &mut total, &mut innum);
    // join, fixing "0 . 5" from spoken decimals
    let mut e = out.join(" ").replace(" . ", ".");
    while e.ends_with(['+', '-', '*', '/', '^', ' ']) { e.pop(); }
    let e = e.trim().to_string();
    if e.is_empty() || !e.chars().any(|c| c.is_ascii_digit()) { return None; }
    eval_text(&e).map(|_| e)
}

/// Pulls a math expression out of a model answer (drops code fences, "=" parts and words around it).
pub fn expr_from_model(ans: &str) -> Option<String> {
    for line in ans.lines().rev().chain(ans.lines()) {
        let l = line.trim().trim_matches('`').trim();
        let l = l.split('=').next().unwrap_or("").trim();
        let l = l.trim_start_matches(|c: char| c.is_alphabetic() || c == ':' || c == ' ').trim().trim_end_matches('.');
        if !l.is_empty() && eval_text(l).is_some() { return Some(l.to_string()); }
    }
    None
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
            for f in ["asin", "acos", "atan", "sinh", "cosh", "tanh", "sin", "cos", "tan", "ln", "logb", "log", "√", "root", "nCr", "nPr"] {
                if rest.starts_with(f) { v.push(Tok::Func(f)); i += f.chars().count(); if cs.get(i) == Some(&'(') { i += 1; } matched = true; break; }
            }
            if matched { continue; }
            v.push(match c {
                '+' | '−' | '×' | '÷' | '^' => Tok::Op(c), '-' => Tok::Op('−'), '*' => Tok::Op('×'), '/' => Tok::Op('÷'),
                '(' => Tok::LParen, ')' => Tok::RParen, ';' => Tok::Sep, '<' | '>' | '≤' | '≥' => Tok::Cmp(c), 'm' => Tok::Op('m'), 'π' => Tok::Pi, 'e' => Tok::E, 'x' => Tok::X, '=' => Tok::Equals,
                '²' => Tok::Post("²"), '!' => Tok::Post("!"), '%' => Tok::Post("%"), '⁻' => { i += 1; Tok::Post("⁻¹") }
                _ => panic!("bad {c}"),
            });
            i += 1;
        }
        v
    }
    fn ev(s: &str) -> Result<f64, CalcError> { Evaluator::new(true).eval(&toks(s)) }
    fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

    #[test] fn text_problems() {
        assert_eq!(text_to_expr("du plius du").as_deref(), Some("2 + 2"));
        assert!(close(eval_text(&text_to_expr("what is three hundred and twenty times four").unwrap()).unwrap(), 1280.0));
        assert!(close(eval_text(&text_to_expr("15 procentų nuo 200").unwrap()).unwrap(), 30.0));
        assert!(close(eval_text(&text_to_expr("dvidešimt penki padalinti iš penki").unwrap()).unwrap(), 5.0));
        assert!(close(eval_text("2*(3+4)^2").unwrap(), 98.0));
        assert!(close(eval_text("sqrt(16) + 1,5").unwrap(), 5.5));
        assert_eq!(expr_from_model("```\n12*3+4\n```").as_deref(), Some("12*3+4"));
        assert_eq!(expr_from_model("Expression: (5+7)/2 = 6").as_deref(), Some("(5+7)/2"));
        assert!(text_to_expr("labas").is_none());
    }
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
        assert!(close(Evaluator::new(false).eval(&toks("sin(π÷2)")).unwrap(), 1.0));
    }
    #[test] fn percent() {
        assert_eq!(ev("50%"), Ok(0.5));
        assert_eq!(ev("200+10%"), Ok(220.0));
        assert_eq!(ev("200−10%"), Ok(180.0));
        assert_eq!(ev("200×10%"), Ok(20.0));
    }
    #[test] fn solving() {
        let s = |q: &str| Evaluator::new(true).solve(&toks(q));
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

    #[test] fn new_functions() {
        assert!(close(ev("asin(0.5)").unwrap(), 30.0));
        assert!(close(ev("atan(1)").unwrap(), 45.0));
        assert!(close(ev("cosh(0)").unwrap(), 1.0));
        assert_eq!(ev("root(3;27)"), Ok(3.0));
        assert_eq!(ev("root(3;−8)"), Ok(-2.0));
        assert_eq!(ev("logb(2;8)"), Ok(3.0));
        assert_eq!(ev("nCr(5;2)"), Ok(10.0));
        assert_eq!(ev("nPr(5;2)"), Ok(20.0));
        assert_eq!(ev("17m5"), Ok(2.0));
        assert_eq!(Evaluator::new(true).eval(&[Tok::Num("1.5e3".into()), Tok::Op('+'), Tok::Num("1".into())]), Ok(1501.0));
        assert_eq!(ev("0.1+0.2−0.3"), Ok(0.0));
    }
    #[test] fn cubic_and_ineq() {
        let e = Evaluator::new(true);
        assert_eq!(e.solve(&toks("x^3−6x²+11x−6=0")), Ok(vec![1.0, 2.0, 3.0]));
        assert_eq!(e.solve(&toks("x^3=8")), Ok(vec![2.0]));
        let r = e.solve(&toks("sin(x)=0.5")).unwrap();
        assert!(r.iter().any(|v| close(*v, 30.0)));
        assert_eq!(e.solve_ineq(&toks("2x+3>7")), Ok("x > 2".into()));
        assert_eq!(e.solve_ineq(&toks("x²−4x+3<0")), Ok("1 < x < 3".into()));
    }
    #[test] fn fractions_words_numbers() {
        assert_eq!(fraction_text(1.0 / 3.0 + 1.0 / 6.0), Some("1/2".into()));
        assert_eq!(fraction_text(7.0 / 4.0), Some("1 3/4".into()));
        assert_eq!(fraction_text(std::f64::consts::PI), None);
        assert_eq!(gcd(12, 18), 6); assert_eq!(lcm(4, 6), Some(12));
        assert_eq!(factorize(360), vec![(2, 3), (3, 2), (5, 1)]);
        assert_eq!(words(1234.0, 1).unwrap(), "vienas tūkstantis du šimtai trisdešimt keturi");
        assert_eq!(words(21.5, 0).unwrap(), "twenty-one point five");
        assert_eq!(words(12000.0, 1).unwrap(), "dvylika tūkstančių");
        assert_eq!(display_num(1.5e-12), "1,5E−12");
        assert_eq!(parse_pasted("1.234,50 €"), Some("1234.50".into()));
        assert!(close(integrate(&|x| Some(x * x), 0.0, 3.0).unwrap(), 9.0));
    }
}
