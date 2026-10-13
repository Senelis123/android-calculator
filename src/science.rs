//! Extra scientific functions, constants and unit conversions for the scientific keypad,
//! plus the data behind the formula calculators, constants list and periodic table.

use crate::expr::CalcError;

pub const SCI_PAGES: usize = 6;

const PAGE2: [&str; 20] = [
    "4th", "eˣ", "10ˣ", "2ˣ", "log₂", "∛", "abs", "trunc", "sec", "csc", "cot", "asinh", "acosh",
    "atanh", "floor", "ceil", "round", "sign", "Γ", "frac",
];
const PAGE3: [&str; 20] = [
    "5th", "C→F", "F→C", "km→mi", "mi→km", "kg→lb", "lb→kg", "in→cm", "cm→in", "m→ft", "ft→m",
    "L→gal", "gal→L", "kmh→ms", "ms→kmh", "°→rad", "rad→°", "cal→J", "J→cal", "atm→Pa",
];
const PAGE4: [&str; 20] = [
    "6th", "c", "G", "h", "ℏ", "k_B", "N_A", "R", "q_e", "ε₀", "μ₀", "g", "σ_SB", "m_e", "m_p",
    "m_n", "u", "F_c", "a₀", "atm",
];
const PAGE5: [&str; 20] = [
    "1st", "φ", "γ", "√2", "√3", "ln2", "ln10", "☉M", "⊕M", "R⊕", "AU", "ly", "pc", "T₀", "Vm",
    "ρ_w", "c_s", "L☉", "kcal", "hp",
];

/// Keys of scientific page `p` (2 and up; pages 0 and 1 live in the app).
pub fn page_keys(p: usize) -> &'static [&'static str; 20] {
    match p {
        2 => &PAGE2,
        3 => &PAGE3,
        4 => &PAGE4,
        _ => &PAGE5,
    }
}

const EXTRA: &[&str] = &[
    "eˣ", "10ˣ", "2ˣ", "log₂", "∛", "trunc", "sec", "csc", "cot", "asinh", "acosh", "atanh",
    "floor", "ceil", "round", "sign", "Γ", "frac", "C→F", "F→C", "km→mi", "mi→km", "kg→lb",
    "lb→kg", "in→cm", "cm→in", "m→ft", "ft→m", "L→gal", "gal→L", "kmh→ms", "ms→kmh", "°→rad",
    "rad→°", "cal→J", "J→cal", "atm→Pa",
];

pub fn extra_func(k: &str) -> Option<&'static str> {
    EXTRA.iter().find(|f| **f == k).copied()
}
pub fn is_extra(f: &str) -> bool {
    EXTRA.contains(&f)
}

/// Physical and mathematical constants as raw number text (usable in expressions).
pub const CONSTANTS: &[(&str, &str, &str, &str)] = &[
    ("c", "Speed of light", "299792458", "m/s"),
    ("G", "Gravitational constant", "6.6743e-11", "m³/(kg·s²)"),
    ("h", "Planck constant", "6.62607015e-34", "J·s"),
    ("ℏ", "Reduced Planck constant", "1.054571817e-34", "J·s"),
    ("k_B", "Boltzmann constant", "1.380649e-23", "J/K"),
    ("N_A", "Avogadro constant", "6.02214076e23", "1/mol"),
    ("R", "Gas constant", "8.314462618", "J/(mol·K)"),
    ("q_e", "Elementary charge", "1.602176634e-19", "C"),
    ("ε₀", "Vacuum permittivity", "8.8541878128e-12", "F/m"),
    ("μ₀", "Vacuum permeability", "1.25663706212e-6", "N/A²"),
    ("g", "Standard gravity", "9.80665", "m/s²"),
    ("σ_SB", "Stefan-Boltzmann constant", "5.670374419e-8", "W/(m²·K⁴)"),
    ("m_e", "Electron mass", "9.1093837015e-31", "kg"),
    ("m_p", "Proton mass", "1.67262192369e-27", "kg"),
    ("m_n", "Neutron mass", "1.67492749804e-27", "kg"),
    ("u", "Atomic mass unit", "1.66053906660e-27", "kg"),
    ("F_c", "Faraday constant", "96485.33212", "C/mol"),
    ("a₀", "Bohr radius", "5.29177210903e-11", "m"),
    ("atm", "Standard atmosphere", "101325", "Pa"),
    ("φ", "Golden ratio", "1.618033988749895", ""),
    ("γ", "Euler-Mascheroni constant", "0.5772156649015329", ""),
    ("√2", "Square root of 2", "1.4142135623730951", ""),
    ("√3", "Square root of 3", "1.7320508075688772", ""),
    ("ln2", "Natural log of 2", "0.6931471805599453", ""),
    ("ln10", "Natural log of 10", "2.302585092994046", ""),
    ("☉M", "Solar mass", "1.98847e30", "kg"),
    ("⊕M", "Earth mass", "5.9722e24", "kg"),
    ("R⊕", "Earth mean radius", "6371000", "m"),
    ("AU", "Astronomical unit", "149597870700", "m"),
    ("ly", "Light year", "9460730472580800", "m"),
    ("pc", "Parsec", "3.0856775814913673e16", "m"),
    ("T₀", "Zero Celsius", "273.15", "K"),
    ("Vm", "Molar volume of gas (STP)", "0.02241396954", "m³/mol"),
    ("ρ_w", "Density of water", "997", "kg/m³"),
    ("c_s", "Speed of sound in air (20 °C)", "343", "m/s"),
    ("L☉", "Solar luminosity", "3.828e26", "W"),
    ("kcal", "Kilocalorie", "4184", "J"),
    ("hp", "Horsepower", "745.7", "W"),
];

pub fn constant_value(k: &str) -> Option<&'static str> {
    CONSTANTS.iter().find(|c| c.0 == k).map(|c| c.2)
}

fn gamma(x: f64) -> f64 {
    if x < 0.5 {
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x));
    }
    let x = x - 1.0;
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    let mut a = C[0];
    let t = x + G + 0.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    (2.0 * std::f64::consts::PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
}

fn clean(v: f64) -> f64 {
    let r = v.round();
    if (v - r).abs() < 1e-12 * r.abs().max(1.0) {
        r
    } else {
        v
    }
}

pub fn extra_eval(f: &str, x: f64, deg: bool) -> Result<f64, CalcError> {
    let dom = |ok: bool| if ok { Ok(()) } else { Err(CalcError::Domain) };
    let r = |x: f64| if deg { x.to_radians() } else { x };
    let v = match f {
        "eˣ" => x.exp(),
        "10ˣ" => 10f64.powf(x),
        "2ˣ" => 2f64.powf(x),
        "log₂" => {
            dom(x > 0.0)?;
            clean(x.log2())
        }
        "∛" => x.cbrt(),
        "trunc" => x.trunc(),
        "sec" | "csc" | "cot" => {
            let (s, c) = (r(x).sin(), r(x).cos());
            let d = match f {
                "sec" => c,
                "csc" => s,
                _ => s,
            };
            dom(d.abs() > 1e-12)?;
            match f {
                "sec" => 1.0 / c,
                "csc" => 1.0 / s,
                _ => c / s,
            }
        }
        "asinh" => x.asinh(),
        "acosh" => {
            dom(x >= 1.0)?;
            x.acosh()
        }
        "atanh" => {
            dom(x.abs() < 1.0)?;
            x.atanh()
        }
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "sign" => {
            if x == 0.0 {
                0.0
            } else {
                x.signum()
            }
        }
        "frac" => x - x.trunc(),
        "Γ" => {
            dom(x > 0.0 || x.fract() != 0.0)?;
            dom(x < 171.0)?;
            clean(gamma(x))
        }
        "C→F" => x * 9.0 / 5.0 + 32.0,
        "F→C" => (x - 32.0) * 5.0 / 9.0,
        "km→mi" => x / 1.609344,
        "mi→km" => x * 1.609344,
        "kg→lb" => x / 0.45359237,
        "lb→kg" => x * 0.45359237,
        "in→cm" => x * 2.54,
        "cm→in" => x / 2.54,
        "m→ft" => x / 0.3048,
        "ft→m" => x * 0.3048,
        "L→gal" => x / 3.785411784,
        "gal→L" => x * 3.785411784,
        "kmh→ms" => x / 3.6,
        "ms→kmh" => x * 3.6,
        "°→rad" => x.to_radians(),
        "rad→°" => x.to_degrees(),
        "cal→J" => x * 4.184,
        "J→cal" => x / 4.184,
        "atm→Pa" => x * 101325.0,
        _ => return Err(CalcError::Syntax),
    };
    if v.is_finite() {
        Ok(v)
    } else {
        Err(CalcError::Domain)
    }
}

// ---------------- formula calculators (used by tools.rs) ----------------

pub const GEO_MODES: &[&str] = &[
    "Circle", "Rectangle", "Triangle (3 sides)", "Triangle (base, height)", "Trapezoid",
    "Parallelogram", "Sphere", "Cylinder", "Cone", "Cube", "Box", "Square pyramid",
    "Regular polygon", "Ellipse", "Circle sector",
];
/// (field labels shown for each geometry mode)
pub fn geo_labels(mode: usize) -> &'static [&'static str] {
    match mode {
        0 | 6 => &["Radius r"],
        1 => &["Side a", "Side b"],
        2 => &["Side a", "Side b", "Side c"],
        3 => &["Base b", "Height h"],
        4 => &["Base a", "Base b", "Height h"],
        5 => &["Side a", "Height h"],
        7 | 8 => &["Radius r", "Height h"],
        9 => &["Edge a"],
        10 => &["Length a", "Width b", "Height c"],
        11 => &["Base edge a", "Height h"],
        12 => &["Sides n", "Side length a"],
        13 => &["Semi-axis a", "Semi-axis b"],
        _ => &["Radius r", "Angle °"],
    }
}

pub const PHYS_MODES: &[&str] = &[
    "v = s / t", "F = m·a", "Ek = ½mv²", "Ep = mgh", "W = F·s", "P = W / t", "U = I·R", "I = U / R",
    "P = U·I", "ρ = m / V", "p = F / A", "p = ρgh", "Q = mcΔT", "v = v₀ + at", "s = v₀t + ½at²",
    "Gravity F = Gm₁m₂/r²", "Coulomb F = kq₁q₂/r²", "Pendulum T = 2π√(l/g)", "λ = v / f",
    "E = mc²", "E = hf",
];
pub fn phys_labels(mode: usize) -> &'static [&'static str] {
    match mode {
        0 => &["Distance s (m)", "Time t (s)"],
        1 => &["Mass m (kg)", "Acceleration a (m/s²)"],
        2 => &["Mass m (kg)", "Speed v (m/s)"],
        3 => &["Mass m (kg)", "Height h (m)"],
        4 => &["Force F (N)", "Distance s (m)"],
        5 => &["Work W (J)", "Time t (s)"],
        6 | 8 => &["Current I (A)", "Resistance R (Ω)"],
        7 => &["Voltage U (V)", "Resistance R (Ω)"],
        9 => &["Mass m (kg)", "Volume V (m³)"],
        10 => &["Force F (N)", "Area A (m²)"],
        11 => &["Density ρ (kg/m³)", "Depth h (m)"],
        12 => &["Mass m (kg)", "Specific heat c (J/kg·K)", "ΔT (K)"],
        13 | 14 => &["Start speed v₀ (m/s)", "Acceleration a (m/s²)", "Time t (s)"],
        15 => &["Mass m₁ (kg)", "Mass m₂ (kg)", "Distance r (m)"],
        16 => &["Charge q₁ (C)", "Charge q₂ (C)", "Distance r (m)"],
        17 => &["Length l (m)"],
        18 => &["Speed v (m/s)", "Frequency f (Hz)"],
        19 => &["Mass m (kg)"],
        _ => &["Frequency f (Hz)"],
    }
}

pub const CHEM_MODES: &[&str] = &[
    "n = m / M", "m = n·M", "c = n / V", "PV = nRT → P", "PV = nRT → V", "pH from [H⁺]",
    "[H⁺] from pH", "Dilution C₁V₁ = C₂V₂", "Molarity from mass", "Particles N = n·N_A",
    "Gas volume at STP", "Percent yield",
];
pub fn chem_labels(mode: usize) -> &'static [&'static str] {
    match mode {
        0 => &["Mass m (g)", "Molar mass M (g/mol)"],
        1 => &["Amount n (mol)", "Molar mass M (g/mol)"],
        2 => &["Amount n (mol)", "Volume V (L)"],
        3 => &["Amount n (mol)", "Temperature T (K)", "Volume V (L)"],
        4 => &["Amount n (mol)", "Temperature T (K)", "Pressure P (kPa)"],
        5 => &["[H⁺] (mol/L)"],
        6 => &["pH"],
        7 => &["C₁ (mol/L)", "V₁ (L)", "C₂ (mol/L)"],
        8 => &["Mass m (g)", "Molar mass M (g/mol)", "Volume V (L)"],
        9 | 10 => &["Amount n (mol)"],
        _ => &["Actual yield", "Theoretical yield"],
    }
}

fn out(label: &str, v: f64, unit: &str) -> (String, String) {
    let n = crate::expr::display_num(v);
    (label.to_string(), if unit.is_empty() { n } else { format!("{n} {unit}") })
}

pub fn geometry(mode: usize, v: &[f64]) -> Vec<(String, String)> {
    use std::f64::consts::PI;
    let g = |i: usize| v.get(i).copied().filter(|x| x.is_finite() && *x >= 0.0);
    let mut o = vec![];
    let need = geo_labels(mode).len();
    if (0..need).any(|i| g(i).is_none()) {
        return o;
    }
    let (a, b, c) = (g(0).unwrap_or(0.0), g(1).unwrap_or(0.0), g(2).unwrap_or(0.0));
    match mode {
        0 => {
            o.push(out("Area", PI * a * a, ""));
            o.push(out("Circumference", 2.0 * PI * a, ""));
            o.push(out("Diameter", 2.0 * a, ""));
        }
        1 => {
            o.push(out("Area", a * b, ""));
            o.push(out("Perimeter", 2.0 * (a + b), ""));
            o.push(out("Diagonal", a.hypot(b), ""));
        }
        2 => {
            if a + b <= c || a + c <= b || b + c <= a {
                o.push(("Result".into(), "Not a triangle".into()));
            } else {
                let p = (a + b + c) / 2.0;
                let area = (p * (p - a) * (p - b) * (p - c)).sqrt();
                o.push(out("Area", area, ""));
                o.push(out("Perimeter", a + b + c, ""));
                let ang = |x: f64, y: f64, z: f64| ((y * y + z * z - x * x) / (2.0 * y * z)).clamp(-1.0, 1.0).acos().to_degrees();
                o.push(out("Angle A", ang(a, b, c), "°"));
                o.push(out("Angle B", ang(b, a, c), "°"));
                o.push(out("Angle C", ang(c, a, b), "°"));
                o.push(out("Inradius", area / p, ""));
                o.push(out("Circumradius", a * b * c / (4.0 * area), ""));
            }
        }
        3 => o.push(out("Area", a * b / 2.0, "")),
        4 => o.push(out("Area", (a + b) / 2.0 * c, "")),
        5 => o.push(out("Area", a * b, "")),
        6 => {
            o.push(out("Volume", 4.0 / 3.0 * PI * a.powi(3), ""));
            o.push(out("Surface area", 4.0 * PI * a * a, ""));
        }
        7 => {
            o.push(out("Volume", PI * a * a * b, ""));
            o.push(out("Lateral surface", 2.0 * PI * a * b, ""));
            o.push(out("Total surface", 2.0 * PI * a * (a + b), ""));
        }
        8 => {
            let l = a.hypot(b);
            o.push(out("Volume", PI * a * a * b / 3.0, ""));
            o.push(out("Slant height", l, ""));
            o.push(out("Total surface", PI * a * (a + l), ""));
        }
        9 => {
            o.push(out("Volume", a.powi(3), ""));
            o.push(out("Surface area", 6.0 * a * a, ""));
            o.push(out("Space diagonal", a * 3f64.sqrt(), ""));
        }
        10 => {
            o.push(out("Volume", a * b * c, ""));
            o.push(out("Surface area", 2.0 * (a * b + b * c + a * c), ""));
            o.push(out("Space diagonal", (a * a + b * b + c * c).sqrt(), ""));
        }
        11 => {
            let sl = (b * b + a * a / 4.0).sqrt();
            o.push(out("Volume", a * a * b / 3.0, ""));
            o.push(out("Total surface", a * a + 2.0 * a * sl, ""));
        }
        12 => {
            if a >= 3.0 && a.fract() == 0.0 && b > 0.0 {
                o.push(out("Area", a * b * b / (4.0 * (PI / a).tan()), ""));
                o.push(out("Perimeter", a * b, ""));
                o.push(out("Interior angle", (a - 2.0) * 180.0 / a, "°"));
            }
        }
        13 => {
            o.push(out("Area", PI * a * b, ""));
            let h = ((a - b) / (a + b)).powi(2);
            o.push(out("Perimeter (approx.)", PI * (a + b) * (1.0 + 3.0 * h / (10.0 + (4.0 - 3.0 * h).sqrt())), ""));
        }
        _ => {
            o.push(out("Sector area", PI * a * a * b / 360.0, ""));
            o.push(out("Arc length", 2.0 * PI * a * b / 360.0, ""));
        }
    }
    o
}

pub fn physics(mode: usize, v: &[f64]) -> Vec<(String, String)> {
    let g = |i: usize| v.get(i).copied().filter(|x| x.is_finite());
    let need = phys_labels(mode).len();
    if (0..need).any(|i| g(i).is_none()) {
        return vec![];
    }
    let (a, b, c) = (g(0).unwrap_or(0.0), g(1).unwrap_or(0.0), g(2).unwrap_or(0.0));
    let nz = |x: f64| x != 0.0;
    let mut o = vec![];
    match mode {
        0 if nz(b) => o.push(out("Speed v", a / b, "m/s")),
        1 => o.push(out("Force F", a * b, "N")),
        2 => o.push(out("Kinetic energy", 0.5 * a * b * b, "J")),
        3 => o.push(out("Potential energy", a * 9.80665 * b, "J")),
        4 => o.push(out("Work W", a * b, "J")),
        5 if nz(b) => o.push(out("Power P", a / b, "W")),
        6 => {
            o.push(out("Voltage U", a * b, "V"));
            o.push(out("Power P", a * a * b, "W"));
        }
        7 if nz(b) => {
            o.push(out("Current I", a / b, "A"));
            o.push(out("Power P", a * a / b, "W"));
        }
        8 => o.push(out("Power P", a * b, "W")),
        9 if nz(b) => o.push(out("Density ρ", a / b, "kg/m³")),
        10 if nz(b) => o.push(out("Pressure p", a / b, "Pa")),
        11 => o.push(out("Pressure p", a * 9.80665 * b, "Pa")),
        12 => o.push(out("Heat Q", a * b * c, "J")),
        13 => o.push(out("Speed v", a + b * c, "m/s")),
        14 => o.push(out("Distance s", a * c + 0.5 * b * c * c, "m")),
        15 if nz(c) => o.push(out("Force F", 6.6743e-11 * a * b / (c * c), "N")),
        16 if nz(c) => o.push(out("Force F", 8.9875517923e9 * a * b / (c * c), "N")),
        17 if a > 0.0 => {
            let t = 2.0 * std::f64::consts::PI * (a / 9.80665).sqrt();
            o.push(out("Period T", t, "s"));
            o.push(out("Frequency f", 1.0 / t, "Hz"));
        }
        18 if nz(b) => o.push(out("Wavelength λ", a / b, "m")),
        19 => o.push(out("Energy E", a * 299792458f64.powi(2), "J")),
        20 => {
            o.push(out("Energy E", 6.62607015e-34 * a, "J"));
            o.push(out("Energy E", 6.62607015e-34 * a / 1.602176634e-19, "eV"));
        }
        _ => {}
    }
    o
}

pub fn chemistry(mode: usize, v: &[f64]) -> Vec<(String, String)> {
    let g = |i: usize| v.get(i).copied().filter(|x| x.is_finite());
    let need = chem_labels(mode).len();
    if (0..need).any(|i| g(i).is_none()) {
        return vec![];
    }
    let (a, b, c) = (g(0).unwrap_or(0.0), g(1).unwrap_or(0.0), g(2).unwrap_or(0.0));
    let nz = |x: f64| x != 0.0;
    let mut o = vec![];
    match mode {
        0 if nz(b) => o.push(out("Amount n", a / b, "mol")),
        1 => o.push(out("Mass m", a * b, "g")),
        2 if nz(b) => o.push(out("Concentration c", a / b, "mol/L")),
        3 if nz(c) => {
            let p = a * 8.314462618 * b / c; // kPa when V is in L
            o.push(out("Pressure P", p, "kPa"));
            o.push(out("Pressure P", p / 101.325, "atm"));
        }
        4 if nz(c) => o.push(out("Volume V", a * 8.314462618 * b / c, "L")),
        5 if a > 0.0 => {
            o.push(out("pH", -a.log10(), ""));
            o.push(out("pOH", 14.0 + a.log10(), ""));
        }
        6 => o.push(out("[H⁺]", 10f64.powf(-a), "mol/L")),
        7 if nz(c) => o.push(out("Final volume V₂", a * b / c, "L")),
        8 if nz(b) && nz(c) => o.push(out("Molarity c", a / b / c, "mol/L")),
        9 => o.push(out("Particles N", a * 6.02214076e23, "")),
        10 => o.push(out("Volume at STP", a * 22.41396954, "L")),
        11 if nz(b) => o.push(out("Percent yield", a / b * 100.0, "%")),
        _ => {}
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gamma_values() {
        assert_eq!(extra_eval("Γ", 5.0, true).unwrap(), 24.0);
        assert!((extra_eval("Γ", 0.5, true).unwrap() - std::f64::consts::PI.sqrt()).abs() < 1e-9);
    }
    #[test]
    fn conversions() {
        assert!((extra_eval("C→F", 100.0, true).unwrap() - 212.0).abs() < 1e-9);
        assert!((extra_eval("km→mi", 1.609344, true).unwrap() - 1.0).abs() < 1e-9);
        assert!((extra_eval("sec", 60.0, true).unwrap() - 2.0).abs() < 1e-9);
    }
    #[test]
    fn every_page_key_is_known() {
        for p in 2..SCI_PAGES {
            for k in page_keys(p).iter().skip(1) {
                assert!(extra_func(k).is_some() || constant_value(k).is_some() || *k == "abs", "{k}");
            }
        }
    }
}
