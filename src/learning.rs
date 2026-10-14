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
pub fn question(seed: u64, mode: usize) -> Question {
    let n = (seed % 19 + 2) as i64;
    let m = ((seed / 23) % 12 + 1) as i64;
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
            text: format!("Which element has the symbol {}?", e.0),
            options,
            answer: 0,
            explanation: format!("{} = {}, atomic number {}", e.0, e.1, z),
        };
    }
    let (text, answer, explanation) = if mode == 1 {
        (
            format!("{m}x + {n} = {}. Find x.", m * 4 + n),
            4,
            format!("Subtract {n}, then divide by {m}: x = 4"),
        )
    } else {
        match seed % 3 {
            0 => (
                format!("{n} + {m} = ?"),
                n + m,
                "Add the two numbers".into(),
            ),
            1 => (
                format!("{n} × {m} = ?"),
                n * m,
                "Multiply the two numbers".into(),
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
