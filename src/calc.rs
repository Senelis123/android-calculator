//! Calculator logic, ported 1:1 from the original Java `MainActivity`.

pub const KEYS: [&str; 20] = [
    "C", "⌫", "÷", "×",
    "7", "8", "9", "−",
    "4", "5", "6", "+",
    "1", "2", "3", "=",
    "0", ".", "±", "%",
];

#[derive(Debug, Clone)]
pub struct Calculator {
    current: String,
    first: f64,
    operator: String,
    reset_on_digit: bool,
}

impl Default for Calculator {
    fn default() -> Self {
        Calculator { current: "0".into(), first: f64::NAN, operator: String::new(), reset_on_digit: false }
    }
}

impl Calculator {
    pub fn new() -> Self { Self::default() }

    /// Text shown on the display.
    pub fn display(&self) -> &str { &self.current }

    pub fn press(&mut self, key: &str) {
        match key {
            "C" => {
                self.current = "0".into();
                self.first = f64::NAN;
                self.operator.clear();
                self.reset_on_digit = false;
            }
            "⌫" => {
                if !self.reset_on_digit {
                    // Java String.length()/substring work on UTF-16 units; display text is ASCII.
                    if self.current.chars().count() > 1 {
                        self.current.pop();
                    } else {
                        self.current = "0".into();
                    }
                }
            }
            "." => {
                if self.reset_on_digit {
                    self.current = "0.".into();
                    self.reset_on_digit = false;
                } else if !self.current.contains('.') {
                    self.current.push('.');
                }
            }
            "±" => {
                if self.current != "0" {
                    self.current = match self.current.strip_prefix('-') {
                        Some(rest) => rest.to_string(),
                        None => format!("-{}", self.current),
                    };
                }
            }
            "%" => self.current = format(parse(&self.current) / 100.0),
            "+" | "−" | "×" | "÷" => self.select_operator(key),
            "=" => self.calculate(),
            _ => self.digit(key),
        }
    }

    fn digit(&mut self, key: &str) {
        if self.reset_on_digit {
            self.current = key.into();
            self.reset_on_digit = false;
        } else if self.current == "0" {
            self.current = key.into();
        } else {
            self.current.push_str(key);
        }
    }

    fn select_operator(&mut self, op: &str) {
        if !self.first.is_nan() && !self.operator.is_empty() && !self.reset_on_digit {
            self.calculate();
        }
        self.first = parse(&self.current);
        self.operator = op.into();
        self.reset_on_digit = true;
    }

    fn calculate(&mut self) {
        if self.operator.is_empty() || self.first.is_nan() { return; }
        let second = parse(&self.current);
        let result = match self.operator.as_str() {
            "+" => self.first + second,
            "−" => self.first - second,
            "×" => self.first * second,
            "÷" => {
                if second == 0.0 {
                    self.current = "Error".into();
                    self.first = f64::NAN;
                    self.operator.clear();
                    self.reset_on_digit = true;
                    return;
                }
                self.first / second
            }
            _ => return,
        };
        self.current = format(result);
        self.first = result;
        self.operator.clear();
        self.reset_on_digit = true;
    }
}

/// Java `Double.parseDouble` with fallback to 0.0 on failure.
fn parse(value: &str) -> f64 {
    let v = value.trim();
    // Java rejects "inf"/"nan" spellings that Rust accepts; only plain decimals reach here anyway.
    if v.is_empty() || v.chars().any(|c| c.is_ascii_alphabetic()) { return 0.0; }
    v.parse::<f64>().unwrap_or(0.0)
}

pub fn format(mut value: f64) -> String {
    if value.is_nan() || value.is_infinite() { return "Error".into(); }
    if value.abs() < 1e-10 { value = 0.0; }
    if value == value.round_ties_even() { return java_fixed(value, 0); }
    let s = java_fixed(value, 10);
    let s = s.trim_end_matches('0');
    s.trim_end_matches('.').to_string()
}

/// Emulates Java `String.format(Locale.US, "%.Nf", v)`: Java rounds the shortest
/// round-trip decimal representation half-up (not the exact binary value).
fn java_fixed(value: f64, prec: usize) -> String {
    let neg = value.is_sign_negative();
    let sci = format!("{:e}", value.abs()); // e.g. "1.2345e-3" (shortest round-trip digits)
    let (mant, exp) = sci.split_once('e').unwrap();
    let exp: i64 = exp.parse().unwrap();
    let digits: Vec<u8> = mant.bytes().filter(|b| b.is_ascii_digit()).map(|b| b - b'0').collect();
    // value = 0.d1d2d3... * 10^(exp+1); position of decimal point after `point` digits
    let point = exp + 1;
    // Build integer part and fraction digits (enough to round)
    let total_frac = prec as i64;
    let keep = point + total_frac; // number of leading digits kept (may be <=0)
    let mut kept: Vec<u8>;
    if keep <= 0 {
        let round_up = keep == 0 && digits[0] >= 5;
        kept = vec![0; (point.max(0) + total_frac) as usize];
        if round_up {
            if kept.is_empty() { kept.push(1); } else { *kept.last_mut().unwrap() = 1; }
            if kept.len() as i64 > point.max(0) + total_frac { /* extra leading digit */ }
        }
    } else {
        let keep = keep as usize;
        kept = digits.iter().cloned().take(keep).collect();
        while kept.len() < keep { kept.push(0); }
        if digits.len() > keep && digits[keep] >= 5 {
            let mut i = kept.len();
            loop {
                if i == 0 { kept.insert(0, 1); break; }
                i -= 1;
                if kept[i] == 9 { kept[i] = 0; } else { kept[i] += 1; break; }
            }
        }
    }
    // Split kept into integer and fraction parts: last `prec` digits are fraction.
    let n = kept.len();
    let (int_part, frac_part) = if n >= prec { kept.split_at(n - prec) } else { (&[][..], &kept[..]) };
    let mut int_s: String = int_part.iter().map(|d| (b'0' + d) as char).collect();
    let int_s_trim = int_s.trim_start_matches('0').to_string();
    int_s = if int_s_trim.is_empty() { "0".into() } else { int_s_trim };
    let mut frac_s: String = frac_part.iter().map(|d| (b'0' + d) as char).collect();
    while frac_s.len() < prec { frac_s.insert(0, '0'); }
    let mut out = String::new();
    if neg { out.push('-'); }
    out.push_str(&int_s);
    if prec > 0 { out.push('.'); out.push_str(&frac_s); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(keys: &[&str]) -> String {
        let mut c = Calculator::new();
        for k in keys { c.press(k); }
        c.display().to_string()
    }

    #[test] fn basic_ops() {
        assert_eq!(run(&["1", "2", "+", "7", "="]), "19");
        assert_eq!(run(&["9", "−", "1", "2", "="]), "-3");
        assert_eq!(run(&["6", "×", "7", "="]), "42");
        assert_eq!(run(&["1", "÷", "4", "="]), "0.25");
        assert_eq!(run(&["1", "÷", "3", "="]), "0.3333333333");
        assert_eq!(run(&["2", "÷", "3", "="]), "0.6666666667");
    }
    #[test] fn chaining_and_operator_replace() {
        assert_eq!(run(&["2", "+", "3", "×", "4", "="]), "20"); // left-to-right like Java version
        assert_eq!(run(&["2", "+", "×", "4", "="]), "8");       // operator replaced, not applied
        assert_eq!(run(&["5", "+", "5", "=", "+", "1", "="]), "11");
        assert_eq!(run(&["5", "+", "5", "=", "="]), "10");      // repeated = does nothing
    }
    #[test] fn decimals() {
        assert_eq!(run(&["0", ".", "1", "+", "0", ".", "2", "="]), "0.3");
        assert_eq!(run(&[".", "5"]), "0.5");
        assert_eq!(run(&["1", ".", ".", "5"]), "1.5");
        assert_eq!(run(&["3", "+", ".", "5", "="]), "3.5");
    }
    #[test] fn percent_sign_backspace() {
        assert_eq!(run(&["5", "0", "%"]), "0.5");
        assert_eq!(run(&["5", "±"]), "-5");
        assert_eq!(run(&["5", "±", "±"]), "5");
        assert_eq!(run(&["0", "±"]), "0");
        assert_eq!(run(&["1", "2", "3", "⌫"]), "12");
        assert_eq!(run(&["7", "⌫"]), "0");
        assert_eq!(run(&["2", "+", "3", "=", "⌫"]), "5"); // backspace ignored on a result
        assert_eq!(run(&["5", "±", "⌫"]), "-");           // same as Java
        assert_eq!(run(&["5", "±", "⌫", "="]), "-");
    }
    #[test] fn errors() {
        assert_eq!(run(&["5", "÷", "0", "="]), "Error");
        assert_eq!(run(&["5", "÷", "0", "=", "7"]), "7");
        assert_eq!(run(&["5", "÷", "0", "=", "±"]), "-Error");
        assert_eq!(run(&["5", "÷", "0", "=", "+", "2", "="]), "2");
        assert_eq!(run(&["C"]), "0");
    }
    #[test] fn formatting_matches_java() {
        assert_eq!(format(1e20), "100000000000000000000");
        assert_eq!(format(-0.0), "0");
        assert_eq!(format(1.23456789015), "1.2345678902"); // half-up on shortest repr
        assert_eq!(format(0.00000000005), "0");
        assert_eq!(format(0.99999999999), "1.0000000000".trim_end_matches('0').trim_end_matches('.'));
        assert_eq!(format(-2.5), "-2.5");
        assert_eq!(format(123.456), "123.456");
        assert_eq!(format(1e-10 * 3.0), "0.0000000003");
        assert_eq!(format(f64::INFINITY), "Error");
    }
}
