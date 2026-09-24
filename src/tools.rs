//! Everyday calculators ("Tools"): each tool has numeric input fields, optional modes and computed outputs.

use crate::expr::{display_num, factorize, gcd, lcm};
use crate::i18n::t;

pub struct ToolDef { pub name: &'static str, pub fields: &'static [&'static str], pub modes: &'static [&'static str], pub list: bool }

pub const TOOLS: &[ToolDef] = &[
    ToolDef { name: "Percentages", fields: &["Value", "Percent / new value"], modes: &["Discount", "Markup", "Change %", "X is what % of Y"], list: false },
    ToolDef { name: "Discount", fields: &["Price", "Discount %", "Extra discount %"], modes: &[], list: false },
    ToolDef { name: "Loan / leasing", fields: &["Amount", "Interest % per year", "Years", "Down payment"], modes: &["Annuity", "Linear"], list: false },
    ToolDef { name: "Savings deposit", fields: &["Deposit", "Interest % per year", "Years", "Monthly top-up"], modes: &["Monthly compounding", "Yearly compounding"], list: false },
    ToolDef { name: "Salary (Lithuania)", fields: &["Gross salary €", "Pension fund %"], modes: &["Gross → net", "Net → gross"], list: false },
    ToolDef { name: "Fuel and trip cost", fields: &["Distance km", "Consumption l/100 km", "Fuel price €/l", "People"], modes: &[], list: false },
    ToolDef { name: "Split the bill", fields: &["Amount"], modes: &["Tip 0%", "Tip 5%", "Tip 10%", "Tip 15%"], list: true },
    ToolDef { name: "BMI", fields: &["Weight kg", "Height cm"], modes: &[], list: false },
    ToolDef { name: "Time calculator", fields: &["Hours", "Minutes", "Hours 2", "Minutes 2"], modes: &["Add", "Subtract"], list: false },
    ToolDef { name: "Date calculator", fields: &["Year", "Month", "Day", "Year 2", "Month 2", "Day 2"], modes: &["Days between", "Age", "Add days"], list: false },
    ToolDef { name: "Statistics", fields: &["Value"], modes: &[], list: true },
    ToolDef { name: "GCD / LCM / primes", fields: &["Number A", "Number B"], modes: &[], list: false },
    ToolDef { name: "Number systems", fields: &["Number"], modes: &["From decimal", "From binary", "From octal", "From hex"], list: false },
    ToolDef { name: "Systems of equations", fields: &["a1", "b1", "c1", "d1", "a2", "b2", "c2", "d2", "a3", "b3", "c3", "d3"], modes: &["2 unknowns", "3 unknowns"], list: false },
    ToolDef { name: "Matrices", fields: &["a11", "a12", "a13", "a21", "a22", "a23", "a31", "a32", "a33"], modes: &["2×2", "3×3"], list: false },
    ToolDef { name: "Random and dice", fields: &["From", "To"], modes: &["Random number", "Dice", "Coin"], list: false },
];

pub const T_SYSTEMS: usize = 13;
pub const T_MATRIX: usize = 14;
pub const T_BASES: usize = 12;
pub const T_RANDOM: usize = 15;
pub const T_DATES: usize = 9;

/// Fields shown for the current mode (systems / matrices hide unused cells).
pub fn visible_fields(tool: usize, mode: usize) -> Vec<usize> {
    match (tool, mode) {
        (T_SYSTEMS, 0) => vec![0, 1, 3, 4, 5, 7], // a x + b y = d (c unused)
        (T_MATRIX, 0) => vec![0, 1, 3, 4],
        (T_DATES, 1) => vec![0, 1, 2],
        (T_DATES, 2) => vec![0, 1, 2, 3],
        (t, _) => (0..TOOLS[t].fields.len()).collect(),
    }
}

pub fn field_label(tool: usize, mode: usize, f: usize) -> &'static str {
    match (tool, mode, f) {
        (T_SYSTEMS, 0, 3) => "c1",
        (T_SYSTEMS, 0, 7) => "c2",
        (T_DATES, 2, 3) => "Days",
        (T_SYSTEMS, 1, _) | (T_SYSTEMS, 0, _) | (T_MATRIX, _, _) => TOOLS[tool].fields[f],
        _ => TOOLS[tool].fields[f],
    }
}

fn money(v: f64) -> String { format!("{} €", display_num((v * 100.0).round() / 100.0)) }
fn row(a: &str, b: String) -> (String, String) { (t_dyn(a), b) }
fn t_dyn(s: &str) -> String {
    // translate known static labels; dynamic ones pass through
    for tool in TOOLS { for f in tool.fields.iter().chain(tool.modes.iter()) { if *f == s { return t(f).to_string(); } } }
    OUT_LABELS.iter().find(|l| **l == s).map(|l| t(l).to_string()).unwrap_or_else(|| s.to_string())
}

pub const OUT_LABELS: &[&str] = &["Result", "You pay", "You save", "New price", "Change", "Monthly payment", "First payment", "Last payment", "Total paid", "Interest paid",
    "Final amount", "Interest earned", "Net salary", "Gross salary", "Income tax (GPM)", "PSD 6.98%", "VSD 12.52%", "Pension fund", "Tax-free amount (NPD)",
    "Employer cost", "Fuel needed", "Trip cost", "Per person", "Total", "Each pays", "Tip", "BMI", "Category", "Underweight", "Normal weight", "Overweight", "Obesity",
    "Time", "In minutes", "Days", "Weeks", "Age", "Date", "Count", "Sum", "Mean", "Median", "Mode", "Std. deviation (sample)", "Std. deviation (population)", "Min", "Max", "Range",
    "GCD", "LCM", "Prime", "Factors", "Yes", "No", "Decimal", "Binary", "Octal", "Hex", "Determinant", "Inverse", "No single solution", "Enter numbers", "Invalid date", "Dice", "Coin", "Heads", "Tails",
    "Estimate for 2026: 20% income tax monthly; above 36 average wages a year the extra is taxed at 25-32% in the yearly declaration."];

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}
fn valid_date(y: f64, m: f64, d: f64) -> Option<i64> {
    if y.fract() != 0.0 || m.fract() != 0.0 || d.fract() != 0.0 || !(1.0..=12.0).contains(&m) || d < 1.0 { return None; }
    let z = days_from_civil(y as i64, m as i64, d as i64);
    if civil_from_days(z) != (y as i64, m as i64, d as i64) { return None; }
    Some(z)
}
pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    secs.div_euclid(86400)
}
fn date_text(z: i64) -> String { let (y, m, d) = civil_from_days(z); format!("{y}-{m:02}-{d:02}") }

/// Lithuanian net salary for 2026 (monthly withholding).
pub fn lt_net(gross: f64, pension: f64) -> (f64, f64, f64, f64, f64, f64) {
    let npd = if gross <= 1153.0 { 747.0 } else { (747.0 - 0.49 * (gross - 1153.0)).max(0.0) };
    let psd = gross * 0.0698;
    let vsd = gross * 0.1252;
    let pen = gross * pension / 100.0;
    let gpm = ((gross - npd).max(0.0) * 0.20).max(0.0);
    (gross - psd - vsd - pen - gpm, npd, psd, vsd, gpm, pen)
}

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Computes outputs; `v` are field values (NaN = empty), `list` the collected list values, `seed` a random seed.
pub fn compute(tool: usize, mode: usize, v: &[f64], list: &[f64], seed: u64) -> Vec<(String, String)> {
    let g = |i: usize| v.get(i).cloned().filter(|x| x.is_finite());
    let z = |i: usize| g(i).unwrap_or(0.0);
    let n = display_num;
    let mut o = vec![];
    match tool {
        0 => if let (Some(a), Some(b)) = (g(0), g(1)) {
            match mode {
                0 => { o.push(row("You pay", n(a * (1.0 - b / 100.0)))); o.push(row("You save", n(a * b / 100.0))); }
                1 => o.push(row("New price", n(a * (1.0 + b / 100.0)))),
                2 => if a != 0.0 { o.push(row("Change", format!("{} %", n((b - a) / a.abs() * 100.0)))) },
                _ => if b != 0.0 { o.push(row("Result", format!("{} %", n(a / b * 100.0)))) },
            }
        },
        1 => if let (Some(p), Some(d)) = (g(0), g(1)) {
            let after = p * (1.0 - d / 100.0) * (1.0 - z(2) / 100.0);
            o.push(row("You pay", money(after))); o.push(row("You save", money(p - after)));
        },
        2 => if let (Some(a), Some(r), Some(y)) = (g(0), g(1), g(2)) {
            let p = a - z(3);
            let nm = (y * 12.0).round().max(1.0);
            let i = r / 100.0 / 12.0;
            if mode == 0 {
                let m = if i == 0.0 { p / nm } else { p * i / (1.0 - (1.0 + i).powf(-nm)) };
                o.push(row("Monthly payment", money(m))); o.push(row("Total paid", money(m * nm))); o.push(row("Interest paid", money(m * nm - p)));
            } else {
                let base = p / nm;
                let first = base + p * i; let last = base + base * i;
                let interest = i * base * nm * (nm + 1.0) / 2.0;
                o.push(row("First payment", money(first))); o.push(row("Last payment", money(last)));
                o.push(row("Total paid", money(p + interest))); o.push(row("Interest paid", money(interest)));
            }
        },
        3 => if let (Some(p), Some(r), Some(y)) = (g(0), g(1), g(2)) {
            let (k, per) = if mode == 0 { (12.0, 1.0) } else { (1.0, 12.0) };
            let periods = (y * k).round();
            let i = r / 100.0 / k;
            let top = z(3) * per;
            let mut bal = p; let mut paid = p;
            for _ in 0..periods as u64 { bal = bal * (1.0 + i) + top; paid += top; }
            o.push(row("Final amount", money(bal))); o.push(row("Interest earned", money(bal - paid)));
        },
        4 => if let Some(x) = g(0) {
            let pen = z(1);
            let gross = if mode == 0 { x } else {
                // invert by bisection
                let (mut lo, mut hi) = (0.0, x * 3.0 + 1000.0);
                for _ in 0..100 { let m = (lo + hi) / 2.0; if lt_net(m, pen).0 < x { lo = m } else { hi = m } }
                (hi * 100.0).round() / 100.0
            };
            let (net, npd, psd, vsd, gpm, pn) = lt_net(gross, pen);
            if mode == 1 { o.push(row("Gross salary", money(gross))); }
            o.push(row("Net salary", money(net)));
            o.push(row("Tax-free amount (NPD)", money(npd))); o.push(row("Income tax (GPM)", money(gpm)));
            o.push(row("PSD 6.98%", money(psd))); o.push(row("VSD 12.52%", money(vsd)));
            if pn > 0.0 { o.push(row("Pension fund", money(pn))); }
            o.push(row("Employer cost", money(gross * 1.0177)));
            o.push(("".into(), t("Estimate for 2026: 20% income tax monthly; above 36 average wages a year the extra is taxed at 25-32% in the yearly declaration.").into()));
        },
        5 => if let (Some(d), Some(c), Some(p)) = (g(0), g(1), g(2)) {
            let l = d * c / 100.0;
            o.push(row("Fuel needed", format!("{} l", n((l * 100.0).round() / 100.0)))); o.push(row("Trip cost", money(l * p)));
            if let Some(pp) = g(3).filter(|x| *x > 1.0) { o.push(row("Per person", money(l * p / pp))); }
        },
        6 => if !list.is_empty() {
            let tip = [0.0, 5.0, 10.0, 15.0][mode.min(3)] / 100.0;
            let total: f64 = list.iter().sum();
            o.push(row("Total", money(total * (1.0 + tip))));
            if tip > 0.0 { o.push(row("Tip", money(total * tip))); }
            for (k, a) in list.iter().enumerate() { o.push((format!("{} {}", t("Each pays"), k + 1), money(a * (1.0 + tip)))); }
        } else { o.push(row("Enter numbers", String::new())); },
        7 => if let (Some(w), Some(h)) = (g(0), g(1).filter(|h| *h > 0.0)) {
            let b = w / (h / 100.0).powi(2);
            let cat = if b < 18.5 { "Underweight" } else if b < 25.0 { "Normal weight" } else if b < 30.0 { "Overweight" } else { "Obesity" };
            o.push(row("BMI", n((b * 10.0).round() / 10.0))); o.push(row("Category", t_dyn(cat)));
        },
        8 => {
            let a = z(0) * 60.0 + z(1); let b = z(2) * 60.0 + z(3);
            let m = if mode == 0 { a + b } else { a - b };
            let s = if m < 0.0 { "−" } else { "" }; let ma = m.abs();
            o.push(row("Time", format!("{s}{} h {} min", (ma / 60.0).floor(), n(ma % 60.0)))); o.push(row("In minutes", n(m)));
        },
        9 => match mode {
            0 => match (valid_date(z(0), z(1), z(2)), valid_date(z(3), z(4), z(5))) {
                (Some(a), Some(b)) => { let d = b - a; o.push(row("Days", n(d as f64))); o.push(row("Weeks", n(((d as f64 / 7.0) * 10.0).round() / 10.0))); }
                _ => o.push(row("Invalid date", String::new())),
            },
            1 => match valid_date(z(0), z(1), z(2)) {
                Some(b) => {
                    let (ty, tm, td) = civil_from_days(today_days());
                    let mut age = ty - z(0) as i64; if (tm, td) < (z(1) as i64, z(2) as i64) { age -= 1; }
                    o.push(row("Age", n(age as f64))); o.push(row("Days", n((today_days() - b) as f64)));
                }
                None => o.push(row("Invalid date", String::new())),
            },
            _ => match valid_date(z(0), z(1), z(2)) { Some(a) => o.push(row("Date", date_text(a + z(3) as i64))), None => o.push(row("Invalid date", String::new())) },
        },
        10 => if !list.is_empty() {
            let mut s = list.to_vec(); s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let c = s.len() as f64; let sum: f64 = s.iter().sum(); let mean = sum / c;
            let med = if s.len() % 2 == 1 { s[s.len() / 2] } else { (s[s.len() / 2 - 1] + s[s.len() / 2]) / 2.0 };
            let ss: f64 = s.iter().map(|x| (x - mean).powi(2)).sum();
            let mut best = (s[0], 0); let mut i = 0;
            while i < s.len() { let mut j = i; while j < s.len() && s[j] == s[i] { j += 1; } if j - i > best.1 { best = (s[i], j - i); } i = j; }
            o.push(row("Count", n(c))); o.push(row("Sum", n(sum))); o.push(row("Mean", n(mean))); o.push(row("Median", n(med)));
            if best.1 > 1 { o.push(row("Mode", n(best.0))); }
            if s.len() > 1 { o.push(row("Std. deviation (sample)", n((ss / (c - 1.0)).sqrt()))); }
            o.push(row("Std. deviation (population)", n((ss / c).sqrt())));
            o.push(row("Min", n(s[0]))); o.push(row("Max", n(s[s.len() - 1]))); o.push(row("Range", n(s[s.len() - 1] - s[0])));
        } else { o.push(row("Enter numbers", String::new())); },
        11 => {
            let int = |x: Option<f64>| x.filter(|v| *v >= 0.0 && v.fract() == 0.0 && *v < 1e15).map(|v| v as u64);
            if let (Some(a), Some(b)) = (int(g(0)), int(g(1))) {
                o.push(row("GCD", gcd(a, b).to_string()));
                if let Some(l) = lcm(a, b) { o.push(row("LCM", l.to_string())); }
            }
            if let Some(a) = int(g(0)) {
                if a >= 2 {
                    let f = factorize(a);
                    let prime = f.len() == 1 && f[0].1 == 1;
                    o.push((format!("{} ({a})", t("Prime")), t(if prime { "Yes" } else { "No" }).into()));
                    let sup = |e: u32| e.to_string().chars().map(|c| "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(c.to_digit(10).unwrap() as usize).unwrap()).collect::<String>();
                    o.push(row("Factors", f.iter().map(|(p, e)| if *e > 1 { format!("{p}{}", sup(*e)) } else { p.to_string() }).collect::<Vec<_>>().join(" × ")));
                }
            }
        },
        12 => {}, // bases use the raw text; see bases()
        13 => if mode == 0 {
            let (a1, b1, c1, a2, b2, c2) = (z(0), z(1), z(3), z(4), z(5), z(7));
            let d = a1 * b2 - a2 * b1;
            if d.abs() < 1e-12 { o.push(row("No single solution", String::new())); }
            else { o.push(("x".into(), n((c1 * b2 - c2 * b1) / d))); o.push(("y".into(), n((a1 * c2 - a2 * c1) / d))); }
        } else {
            let m = [[z(0), z(1), z(2)], [z(4), z(5), z(6)], [z(8), z(9), z(10)]];
            let r = [z(3), z(7), z(11)];
            let d = det3(&m);
            if d.abs() < 1e-12 { o.push(row("No single solution", String::new())); }
            else {
                for (k, name) in ["x", "y", "z"].iter().enumerate() {
                    let mut mk = m; for i in 0..3 { mk[i][k] = r[i]; }
                    o.push((name.to_string(), n(det3(&mk) / d)));
                }
            }
        },
        14 => {
            let m3 = if mode == 0 { [[z(0), z(1), 0.0], [z(3), z(4), 0.0], [0.0, 0.0, 1.0]] } else { [[z(0), z(1), z(2)], [z(3), z(4), z(5)], [z(6), z(7), z(8)]] };
            let d = det3(&m3);
            o.push(row("Determinant", n(d)));
            if d.abs() > 1e-12 {
                let sz = if mode == 0 { 2 } else { 3 };
                let mut inv = vec![];
                for i in 0..sz { let mut rowv = vec![]; for j in 0..sz {
                    // cofactor of (j, i) / det
                    let mut minor = [[0.0; 2]; 2]; let (mut r, mut c);
                    r = 0; for a in 0..3 { if a == j { continue; } c = 0; for b in 0..3 { if b == i { continue; } if r < 2 && c < 2 { minor[r][c] = m3[a][b]; } c += 1; } r += 1; }
                    let cof = (minor[0][0] * minor[1][1] - minor[0][1] * minor[1][0]) * if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
                    rowv.push(n(cof / d));
                } inv.push(rowv.join("   ")); }
                o.push(row("Inverse", inv.join("\n")));
            }
        },
        15 => {
            let r = (seed >> 11) as f64 / (1u64 << 53) as f64;
            match mode {
                0 => { let (a, b) = (g(0).unwrap_or(1.0), g(1).unwrap_or(100.0)); let (lo, hi) = (a.min(b).ceil(), a.max(b).floor()); if hi >= lo { o.push(row("Result", n(lo + (r * (hi - lo + 1.0)).floor()))); } }
                1 => { let faces = ["⚀", "⚁", "⚂", "⚃", "⚄", "⚅"]; let k = (r * 6.0) as usize; o.push(row("Dice", format!("{}  {}", k + 1, faces[k.min(5)]))); }
                _ => o.push(row("Coin", t(if r < 0.5 { "Heads" } else { "Tails" }).into())),
            }
        },
        _ => {}
    }
    o
}

/// Number-system conversion from the raw typed text (digits 0-9, A-F).
pub fn bases(raw: &str, mode: usize) -> Vec<(String, String)> {
    let radix = [10, 2, 8, 16][mode.min(3)];
    let neg = raw.starts_with('-');
    let digits = raw.trim_start_matches('-');
    let Ok(v) = u64::from_str_radix(digits, radix) else { return vec![] };
    let s = if neg { "−" } else { "" };
    vec![(t("Decimal").into(), format!("{s}{v}")), (t("Binary").into(), format!("{s}{v:b}")), (t("Octal").into(), format!("{s}{v:o}")), (t("Hex").into(), format!("{s}{v:X}"))]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn get(o: &[(String, String)], k: &str) -> String { o.iter().find(|r| r.0 == k).map(|r| r.1.clone()).unwrap_or_default() }
    #[test] fn tools_work() {
        crate::expr::set_format(0, 0, -1);
        let o = compute(4, 0, &[1800.0, 0.0], &[], 0);
        assert_eq!(get(&o, "Income tax (GPM)"), "274,01 €");
        assert_eq!(get(&o, "Tax-free amount (NPD)"), "429,97 €");
        let o = compute(2, 0, &[10000.0, 0.0, 1.0, 0.0], &[], 0);
        assert_eq!(get(&o, "Monthly payment"), "833,33 €");
        let o = compute(13, 0, &[2.0, 1.0, 0.0, 5.0, 1.0, -1.0, 0.0, 1.0], &[], 0);
        assert_eq!(get(&o, "x"), "2"); assert_eq!(get(&o, "y"), "1");
        let o = compute(13, 1, &[1.0, 1.0, 1.0, 6.0, 0.0, 2.0, 5.0, -4.0, 2.0, 5.0, -1.0, 27.0], &[], 0);
        assert_eq!((get(&o, "x"), get(&o, "y"), get(&o, "z")), ("5".into(), "3".into(), "-2".replace('-', "−")));
        let o = compute(10, 0, &[], &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0], 0);
        assert_eq!(get(&o, "Mean"), "5"); assert_eq!(get(&o, "Std. deviation (population)"), "2"); assert_eq!(get(&o, "Mode"), "4");
        let o = compute(14, 0, &[4.0, 7.0, 0.0, 2.0, 6.0], &[], 0);
        assert_eq!(get(&o, "Determinant"), "10");
        assert_eq!(get(&o, "Inverse"), "0,6   −0,7\n−0,2   0,4");
        assert_eq!(bases("FF", 3)[1].1, "11111111");
        assert_eq!(days_from_civil(2026, 9, 24) - days_from_civil(2026, 1, 1), 266);
        let o = compute(9, 0, &[2026.0, 1.0, 1.0, 2026.0, 12.0, 25.0], &[], 0);
        assert_eq!(get(&o, "Days"), "358");
        let o = compute(11, 0, &[360.0, 84.0], &[], 0);
        assert_eq!(get(&o, "GCD"), "12"); assert_eq!(get(&o, "Factors"), "2³ × 3² × 5");
        let o = compute(7, 0, &[70.0, 175.0], &[], 0);
        assert_eq!(get(&o, "BMI"), "22,9");
    }
}
