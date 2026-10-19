//! Offline practice, saved formulas and data validation.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    pub name: String,
    pub expression: String,
}
impl Formula {
    pub fn variables(&self) -> Vec<String> {
        let mut vars = vec![];
        let cs: Vec<char> = self.expression.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            if cs[i].is_ascii_alphabetic() {
                let st = i;
                while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                    i += 1;
                }
                let word: String = cs[st..i].iter().collect();
                let mut j = i;
                while j < cs.len() && cs[j].is_whitespace() {
                    j += 1;
                }
                if cs.get(j) != Some(&'(') && word != "pi" && !vars.contains(&word) {
                    vars.push(word);
                }
            } else {
                i += 1;
            }
        }
        vars
    }
    pub fn evaluate(&self, values: &[(String, f64)]) -> Result<f64, String> {
        let mut out = String::new();
        let cs: Vec<char> = self.expression.chars().collect();
        let mut i = 0;
        while i < cs.len() {
            if cs[i].is_ascii_alphabetic() {
                let st = i;
                while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                    i += 1;
                }
                let word: String = cs[st..i].iter().collect();
                if let Some((_, v)) = values.iter().find(|p| p.0 == word) {
                    if !v.is_finite() {
                        return Err("Enter finite values".into());
                    }
                    out.push_str(&format!("({v})"));
                } else {
                    out.push_str(&word);
                }
            } else {
                out.push(cs[i]);
                i += 1;
            }
        }
        let v = crate::expr::eval_text(&out).ok_or("Check formula and variable values")?;
        if v.is_finite() {
            Ok(v)
        } else {
            Err("Result is not finite".into())
        }
    }
}
pub fn dataset(s: &str) -> Result<Vec<f64>, String> {
    let values: Result<Vec<_>, _> = s
        .split(|c: char| c.is_whitespace() || c == ';' || c == ',')
        .filter(|s| !s.is_empty())
        .map(str::parse::<f64>)
        .collect();
    let values = values
        .map_err(|_| "Use numbers separated by spaces, commas or semicolons; decimal point '.'")?;
    if values.is_empty() || values.len() > 60 || values.iter().any(|v| !v.is_finite()) {
        return Err("Enter 1 to 60 finite numbers".into());
    }
    Ok(values)
}
#[derive(Debug, Clone)]
pub struct Question {
    pub text: String,
    pub options: Vec<String>,
    pub answer: usize,
    pub explanation: String,
}
#[allow(clippy::approx_constant)]
pub fn question(seed: u64, mode: usize) -> Question {
    let n = (seed % 19 + 2) as i64;
    let m = ((seed / 23) % 12 + 1) as i64;
    let tf = |en: &'static str, args: &[&dyn std::fmt::Display]| crate::i18n::tf(en, args);
    if mode == 2 {
        let z = (seed as usize % 118) + 1;
        let e = crate::science::ELEMENTS[z - 1];
        let options = (0..4)
            .map(|i| {
                crate::science::ELEMENTS[(z - 1 + i * 17) % 118]
                    .1
                    .to_string()
            })
            .collect();
        return Question {
            text: tf("Which element has the symbol {}?", &[&e.0]),
            options,
            answer: 0,
            explanation: format!("{} = {}, {} {}", e.0, e.1, crate::i18n::t("atomic number"), z),
        };
    }
    if mode == 3 {
        // physics: v = s/t, F = ma, E = mv^2/2, A = Fs, weight with g ~ 10
        let v = (seed % 16 + 2) as i64;
        let t = ((seed / 7) % 9 + 2) as i64;
        let (text, answer, explanation) = match seed % 5 {
            0 => {
                let (s_, t_) = (v * t, t);
                (tf("A car covers {} m in {} s. Speed v = s/t (m/s)?", &[&s_, &t_]), v,
                 tf("v = s/t = {}/{} = {}", &[&s_, &t_, &v]))
            }
            1 => {
                let a = ((seed / 11) % 8 + 2) as i64;
                (tf("Force on a {} kg mass with acceleration {} m/s²: F = ma (N)?", &[&v, &a]), v * a,
                 tf("F = ma = {} × {} = {}", &[&v, &a, &(v * a)]))
            }
            2 => {
                let vv = ((seed / 13) % 9 + 2) as i64;
                (tf("Kinetic energy of {} kg moving at {} m/s: E = mv²/2 (J)?", &[&v, &vv]), v * vv * vv / 2,
                 tf("E = {} × {}² / 2 = {}", &[&v, &vv, &(v * vv * vv / 2)]))
            }
            3 => {
                let f = ((seed / 17) % 20 + 5) as i64;
                (tf("Work done by {} N over {} m: A = Fs (J)?", &[&f, &v]), f * v,
                 tf("A = Fs = {} × {} = {}", &[&f, &v, &(f * v)]))
            }
            _ => {
                (tf("Weight of {} kg: F = mg, g ≈ 10 m/s² (N)?", &[&v]), v * 10,
                 tf("F = mg = {} × 10 = {}", &[&v, &(v * 10)]))
            }
        };
        let d = ((seed / 29) % 3 + 1) as i64;
        return Question {
            text,
            options: vec![answer, answer + d, answer - d, answer + 2 * d].into_iter().map(|x| x.to_string()).collect(),
            answer: 0,
            explanation,
        };
    }
    if mode == 4 {
        // geometry: areas, perimeters, pythagorean triples, angle sums, circle with pi ~ 3.14
        let a = (seed % 12 + 3) as i64;
        let b = ((seed / 7) % 9 + 2) as i64;
        let triples = [(3, 4, 5), (6, 8, 10), (5, 12, 13), (8, 15, 17), (7, 24, 25), (9, 12, 15)];
        let (text, answer, explanation): (String, i64, String) = match seed % 5 {
            0 => (tf("Rectangle {} by {}: area?", &[&a, &b]), a * b, tf("S = ab = {} × {} = {}", &[&a, &b, &(a * b)])),
            1 => {
                let h = b * 2;
                (tf("Triangle with base {} and height {}: area?", &[&a, &h]), a * b,
                 tf("S = ah/2 = {} × {} / 2 = {}", &[&a, &h, &(a * b)]))
            }
            2 => {
                let (x, y, z) = triples[(seed as usize / 3) % triples.len()];
                (tf("Right triangle legs {} and {}: hypotenuse?", &[&x, &y]), z as i64,
                 tf("c = √({}² + {}²) = {}", &[&x, &y, &z]))
            }
            3 => {
                let a1 = ((seed / 5) % 8 + 2) as i64 * 10;
                let a2 = ((seed / 11) % 7 + 2) as i64 * 10;
                (tf("Triangle angles {}° and {}°: the third angle?", &[&a1, &a2]), 180 - a1 - a2,
                 tf("180° − {}° − {}° = {}°", &[&a1, &a2, &(180 - a1 - a2)]))
            }
            _ => {
                let r = ((seed / 13) % 9 + 1) as i64;
                let c = (2.0 * 3.14 * r as f64 * 10.0).round() as i64;
                (tf("Circle radius {}: circumference C = 2πr, π ≈ 3.14 (in 0.1 units)?", &[&r]), c,
                 tf("C = 2πr = 2 × 3.14 × {} = {}", &[&r, &(c as f64 / 10.0)]))
            }
        };
        let d = ((seed / 29) % 3 + 1) as i64;
        return Question {
            text,
            options: vec![answer, answer + d, answer - d.max(1), answer + 2 * d].into_iter().map(|x| x.to_string()).collect(),
            answer: 0,
            explanation,
        };
    }
    let solution=(seed/41%15+1) as i64;
    let (text, answer, explanation) = if mode == 1 {
        (
            tf("{}x + {} = {}. Find x.", &[&m, &n, &(m * solution + n)]),
            solution,
            tf("Subtract {}, then divide by {}: x = {}", &[&n, &m, &solution]),
        )
    } else {
        match seed % 3 {
            0 => (
                format!("{n} + {m} = ?"),
                n + m,
                crate::i18n::t("Add the two numbers").to_string(),
            ),
            1 => (
                format!("{n} × {m} = ?"),
                n * m,
                crate::i18n::t("Multiply the two numbers").to_string(),
            ),
            _ => (
                format!("{} ÷ {m} = ?", n * m),
                n,
                format!("{n} × {m} = {}", n * m),
            ),
        }
    };
    Question {
        text,
        options: vec![answer, answer + 1, answer + 3, answer - 1]
            .into_iter()
            .map(|v| v.to_string())
            .collect(),
        answer: 0,
        explanation,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formula_names() {
        let f = Formula {
            name: "Area".into(),
            expression: "width*height+sin(angle)".into(),
        };
        assert_eq!(f.variables(), vec!["width", "height", "angle"]);
        assert_eq!(
            f.evaluate(&[
                ("width".into(), 3.),
                ("height".into(), 4.),
                ("angle".into(), 0.)
            ]),
            Ok(12.)
        );
    }
    #[test]
    fn input_validation() {
        assert!(dataset("1 2.5;3,4").is_ok());
        assert!(dataset("NaN 2").is_err());
        assert!(dataset("foo").is_err());
    }
    #[test]
    fn quiz_answers() {
        for n in 0..100 {
            for m in 0..3 {
                let q = question(n, m);
                assert_eq!(q.options.len(), 4);
                assert!(q.answer < 4);
            }
        }
    }
}
