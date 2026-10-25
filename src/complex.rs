//! Complex-number evaluation for expressions that contain i (e.g. (2+3i)×(1−i), √(−4)).

use crate::expr::{display_num, CalcError, Tok};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct C { pub re: f64, pub im: f64 }

impl C {
    fn r(v: f64) -> C { C { re: v, im: 0.0 } }
    fn add(self, o: C) -> C { C { re: self.re + o.re, im: self.im + o.im } }
    fn neg(self) -> C { C { re: -self.re, im: -self.im } }
    fn mul(self, o: C) -> C { C { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re } }
    fn div(self, o: C) -> Result<C, CalcError> {
        let d = o.re * o.re + o.im * o.im;
        if d == 0.0 { return Err(CalcError::DivZero); }
        Ok(C { re: (self.re * o.re + self.im * o.im) / d, im: (self.im * o.re - self.re * o.im) / d })
    }
    fn abs(self) -> f64 { self.re.hypot(self.im) }
    fn ln(self) -> Result<C, CalcError> { if self.abs() == 0.0 { return Err(CalcError::Domain); } Ok(C { re: self.abs().ln(), im: self.im.atan2(self.re) }) }
    fn exp(self) -> C { let m = self.re.exp(); C { re: m * self.im.cos(), im: m * self.im.sin() } }
    fn pow(self, e: C) -> Result<C, CalcError> {
        if self.re == 0.0 && self.im == 0.0 {
            // Preserve 0^0 = 1, but never silently accept undefined powers.
            if e.im != 0.0 || !e.re.is_finite() { return Err(CalcError::Domain); }
            if e.re < 0.0 { return Err(CalcError::DivZero); }
            return Ok(C::r(if e.re == 0.0 { 1.0 } else { 0.0 }));
        }
        if e.im == 0.0 && e.re.fract() == 0.0 && e.re.abs() <= 64.0 {
            let mut acc = C::r(1.0);
            // Invert first for negative powers, avoiding overflow in a positive power.
            let mut base = if e.re < 0.0 { C::r(1.0).div(self)? } else { self };
            let mut n = e.re.abs() as u32;
            while n > 0 {
                if n & 1 != 0 { acc = acc.mul(base); }
                n >>= 1;
                if n > 0 { base = base.mul(base); }
            }
            return Ok(acc);
        }
        Ok(e.mul(self.ln()?).exp())
    }
    fn sqrt(self) -> C { let m = self.abs(); let re = ((m + self.re) / 2.0).sqrt(); let im = ((m - self.re) / 2.0).sqrt(); C { re, im: if self.im < 0.0 { -im } else { im } } }
    /// "3 + 2i" style text.
    pub fn text(&self) -> String {
        let cl = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v };
        let (re, im) = (cl(self.re), cl(self.im));
        if im == 0.0 { return display_num(re); }
        let ims = if im.abs() == 1.0 { String::new() } else { display_num(im.abs()) };
        if re == 0.0 { return format!("{}{}i", if im < 0.0 { "−" } else { "" }, ims); }
        format!("{} {} {}i", display_num(re), if im < 0.0 { "−" } else { "+" }, ims)
    }
}

struct P<'a> { t: &'a [Tok], i: usize, deg: bool, ans: f64 }

impl<'a> P<'a> {
    fn peek(&self) -> Option<&Tok> { self.t.get(self.i) }
    fn expr(&mut self) -> Result<C, CalcError> {
        let mut a = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '−'))) = self.peek().cloned() { self.i += 1; let b = self.term()?; a = if c == '+' { a.add(b) } else { a.add(b.neg()) }; }
        Ok(a)
    }
    fn term(&mut self) -> Result<C, CalcError> {
        let mut a = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op('×')) => { self.i += 1; a = a.mul(self.unary()?); }
                Some(Tok::Op('÷')) => { self.i += 1; a = a.div(self.unary()?)?; }
                Some(Tok::Num(_) | Tok::LParen | Tok::Func(_) | Tok::Pi | Tok::E | Tok::I | Tok::Ans) => { a = a.mul(self.unary()?); }
                _ => break,
            }
        }
        Ok(a)
    }
    fn unary(&mut self) -> Result<C, CalcError> {
        match self.peek() {
            Some(Tok::Op('−')) => { self.i += 1; Ok(self.unary()?.neg()) }
            Some(Tok::Op('+')) => { self.i += 1; self.unary() }
            _ => {
                let b = self.post()?;
                if let Some(Tok::Op('^')) = self.peek() { self.i += 1; let e = self.unary()?; return b.pow(e); }
                Ok(b)
            }
        }
    }
    fn post(&mut self) -> Result<C, CalcError> {
        let mut v = self.prim()?;
        while let Some(Tok::Post(p)) = self.peek().cloned() {
            self.i += 1;
            v = match p { "²" => v.mul(v), "⁻¹" => C::r(1.0).div(v)?, "%" => v.div(C::r(100.0))?, _ => return Err(CalcError::Syntax) };
        }
        Ok(v)
    }
    fn prim(&mut self) -> Result<C, CalcError> {
        let t = self.peek().cloned().ok_or(CalcError::Syntax)?;
        self.i += 1;
        Ok(match t {
            Tok::Num(s) => C::r(s.parse().map_err(|_| CalcError::Syntax)?),
            Tok::Pi => C::r(std::f64::consts::PI),
            Tok::E => C::r(std::f64::consts::E),
            Tok::I => C { re: 0.0, im: 1.0 },
            Tok::Ans => C::r(self.ans),
            Tok::LParen => { let v = self.expr()?; if let Some(Tok::RParen) = self.peek() { self.i += 1; } v }
            Tok::Func(f) => {
                let v = self.expr()?;
                if let Some(Tok::RParen) = self.peek() { self.i += 1; }
                match f {
                    "√" => v.sqrt(),
                    "abs" => C::r(v.abs()),
                    "ln" => v.ln()?,
                    "log" => v.ln()?.div(C::r(10f64.ln()))?,
                    _ if v.im == 0.0 => {
                        let x = if self.deg && matches!(f, "sin" | "cos" | "tan") { v.re.to_radians() } else { v.re };
                        C::r(match f { "sin" => x.sin(), "cos" => x.cos(), "tan" => x.tan(), "sinh" => x.sinh(), "cosh" => x.cosh(), "tanh" => x.tanh(), _ => return Err(CalcError::Syntax) })
                    }
                    _ => return Err(CalcError::NonPoly),
                }
            }
            _ => return Err(CalcError::Syntax),
        })
    }
}

pub fn eval(t: &[Tok], deg: bool, ans: f64) -> Result<C, CalcError> {
    let mut n = t.len();
    while n > 0 && matches!(t[n - 1], Tok::Op(_) | Tok::LParen | Tok::Func(_)) { n -= 1; }
    let mut p = P { t: &t[..n], i: 0, deg, ans };
    let v = p.expr()?;
    while let Some(Tok::RParen) = p.peek() { p.i += 1; }
    if p.i != n || !v.re.is_finite() || !v.im.is_finite() { return Err(CalcError::Syntax); }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn n(v: &str) -> Tok { Tok::Num(v.into()) }
    #[test] fn complex_math() {
        // (2+3i)×(1−i) = 5 + i
        let t = vec![Tok::LParen, n("2"), Tok::Op('+'), n("3"), Tok::I, Tok::RParen, Tok::Op('×'), Tok::LParen, n("1"), Tok::Op('−'), Tok::I, Tok::RParen];
        assert_eq!(eval(&t, true, 0.0).unwrap().text(), "5 + i");
        let t = vec![Tok::I, Tok::Op('^'), n("2")];
        assert_eq!(eval(&t, true, 0.0).unwrap().text(), "−1");
        let t = vec![Tok::Func("√"), Tok::Op('−'), n("4"), Tok::RParen, Tok::Op('+'), Tok::I, Tok::Op('×'), n("0")];
        assert_eq!(eval(&t, true, 0.0).unwrap().text(), "2i");
    }
}


#[cfg(test)]
mod reliability_regression_tests {
    use super::*;

    #[test]
    fn zero_powers_have_explicit_domains() {
        let z = C::r(0.0);
        assert_eq!(z.pow(C::r(0.0)), Ok(C::r(1.0)));
        assert_eq!(z.pow(C::r(0.5)), Ok(z));
        for e in [-0.5, -1.0, -65.0, -100.0] {
            assert_eq!(z.pow(C::r(e)), Err(CalcError::DivZero));
        }
        assert_eq!(z.pow(C { re: 1.0, im: 1.0 }), Err(CalcError::Domain));
    }

    #[test]
    fn integer_powers_match_repeated_multiplication() {
        let z = C { re: 0.75, im: 0.25 };
        let mut expected = C::r(1.0);
        for n in 0..=64 {
            let got = z.pow(C::r(n as f64)).unwrap();
            let tol = 1e-12 * expected.abs().max(1e-100);
            assert!((got.re - expected.re).abs() <= tol);
            assert!((got.im - expected.im).abs() <= tol);
            expected = expected.mul(z);
        }
    }

    #[test]
    fn negative_power_keeps_representable_small_result() {
        let got = C::r(1e6).pow(C::r(-52.0)).unwrap();
        let expected = 1e-312;
        assert!(got.re > 0.0);
        assert!(((got.re - expected) / expected).abs() < 1e-9);
        assert_eq!(got.im, 0.0);
    }

    #[test]
    fn negative_powers_of_i() {
        let i = C { re: 0.0, im: 1.0 };
        assert_eq!(i.pow(C::r(-1.0)), Ok(C { re: 0.0, im: -1.0 }));
        assert_eq!(i.pow(C::r(-2.0)), Ok(C::r(-1.0)));
    }
}
