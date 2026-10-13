//! Step-by-step solutions for equations in x (linear, quadratic, cubic) written as plain text lines.

use crate::expr::{display_num, display_tokens, poly_roots, Evaluator, Poly, SolveError, Tok};

fn num(v: f64) -> String {
    if v.fract() != 0.0 {
        if let Some(f) = crate::expr::fraction_text(v) {
            return f;
        }
    }
    display_num(v)
}

fn signed(v: f64) -> String {
    if v < 0.0 {
        format!("− {}", num(-v))
    } else {
        format!("+ {}", num(v))
    }
}

fn poly_text(c: &[f64; 4]) -> String {
    let names = ["", "x", "x²", "x³"];
    let mut s = String::new();
    for k in (0..4).rev() {
        let a = c[k];
        if a == 0.0 {
            continue;
        }
        let coef = if k > 0 && a.abs() == 1.0 { String::new() } else { num(a.abs()) };
        let term = format!("{}{}", coef, names[k]);
        if s.is_empty() {
            s.push_str(&format!("{}{}", if a < 0.0 { "−" } else { "" }, term));
        } else {
            s.push_str(&format!(" {} {}", if a < 0.0 { "−" } else { "+" }, term));
        }
    }
    if s.is_empty() {
        "0".into()
    } else {
        s
    }
}

/// Explains how to solve the equation held in `toks` (an expression in x, optionally with "=").
pub fn solve_steps(toks: &[Tok], degrees: bool) -> Vec<String> {
    let ev = Evaluator::new(degrees);
    let mut out = vec![format!("1. {}: {}", "Equation", display_tokens(toks))];
    if toks.iter().any(|t| matches!(t, Tok::Cmp(_))) {
        match ev.solve_ineq(toks) {
            Ok(s) => {
                out.push("2. Move everything to one side and find where it changes sign.".into());
                out.push(format!("3. Answer: {s}"));
            }
            Err(_) => out.push("2. Could not solve this inequality.".into()),
        }
        return out;
    }
    let p = match ev.difference(toks) {
        Ok(p) => p,
        Err(_) => {
            out.push("2. This is not a polynomial up to x³, so there are no exact steps. The graph screen shows numeric zeros.".into());
            if let Ok(r) = ev.solve(toks) {
                out.push(format!("Numeric answer: x ≈ {}", r.iter().map(|v| display_num(*v)).collect::<Vec<_>>().join("; ")));
            }
            return out;
        }
    };
    let c = p.0;
    let deg = (0..4).rev().find(|&i| c[i] != 0.0).unwrap_or(0);
    out.push(format!("2. Move everything to the left side: {} = 0", poly_text(&c)));
    match deg {
        0 => {
            if c[0] == 0.0 {
                out.push("3. Both sides are always equal: every x is a solution.".into());
            } else {
                out.push(format!("3. {} = 0 is never true: no solution.", num(c[0])));
            }
        }
        1 => {
            let (a, b) = (c[1], c[0]);
            out.push(format!("3. Linear equation a·x + b = 0 with a = {}, b = {}.", num(a), num(b)));
            if b != 0.0 {
                out.push(format!("4. Move the constant: {}x = {}", if a == 1.0 { String::new() } else { num(a) }, num(-b)));
            }
            if a != 1.0 {
                out.push(format!("5. Divide both sides by {}: x = {} ÷ {}", num(a), num(-b), num(a)));
            }
            out.push(format!("Answer: x = {}", num(-b / a)));
        }
        2 => {
            let (a, b, cc) = (c[2], c[1], c[0]);
            out.push(format!("3. Quadratic a·x² + b·x + c = 0 with a = {}, b = {}, c = {}.", num(a), num(b), num(cc)));
            let disc = b * b - 4.0 * a * cc;
            out.push(format!("4. Discriminant D = b² − 4ac = {}² − 4·{}·{} = {}", num(b), num(a), num(cc), num(disc)));
            if disc < 0.0 {
                out.push("5. D < 0, so there are no real solutions.".into());
            } else if disc == 0.0 {
                out.push(format!("5. D = 0: one solution x = −b / 2a = {}", num(-b / (2.0 * a))));
            } else {
                let sd = disc.sqrt();
                out.push(format!("5. D > 0: two solutions x = (−b ± √D) / 2a, √D = {}", num(sd)));
                out.push(format!("6. x₁ = ({} − {}) / {} = {}", num(-b), num(sd), num(2.0 * a), num((-b - sd) / (2.0 * a))));
                out.push(format!("7. x₂ = ({} + {}) / {} = {}", num(-b), num(sd), num(2.0 * a), num((-b + sd) / (2.0 * a))));
            }
            if let Ok(r) = poly_roots(&Poly(c)) {
                out.push(format!("Answer: x = {}", r.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" or ")));
            } else {
                out.push("Answer: no real solution".into());
            }
        }
        _ => {
            out.push("3. Cubic equation: look for a rational root first (divisors of the constant term), then reduce to a quadratic.".into());
            let r = poly_roots(&Poly(c));
            match r {
                Ok(r) if !r.is_empty() => {
                    let first = r[0];
                    out.push(format!("4. One real root: x = {}", num(first)));
                    out.push(format!("5. Divide by (x {}) and solve the remaining quadratic.", signed(-first)));
                    out.push(format!("Answer: x = {}", r.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" or ")));
                }
                Ok(_) | Err(SolveError::NoRealSolution) => out.push("No real solutions.".into()),
                Err(_) => out.push("Could not solve this equation.".into()),
            }
        }
    }
    out
}
