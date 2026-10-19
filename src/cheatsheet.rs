//! Formula cheat sheet (konspektas): algebra, geometry, trigonometry and physics.
//! Lithuanian is the primary language; English is the fallback for other UI languages.

pub struct Section {
    pub title_lt: &'static str,
    pub title_en: &'static str,
    pub lines: &'static [(&'static str, &'static str)], // (Lithuanian, English)
}

pub const SECTIONS: &[Section] = &[
    Section {
        title_lt: "Algebra",
        title_en: "Algebra",
        lines: &[
            ("Kvadratinė lygtis: ax² + bx + c = 0", "Quadratic equation: ax² + bx + c = 0"),
            ("x = (−b ± √D) / 2a,   D = b² − 4ac", "x = (−b ± √D) / 2a,   D = b² − 4ac"),
            ("D > 0: du sprendiniai; D = 0: vienas; D < 0: nėra realių", "D > 0: two roots; D = 0: one; D < 0: no real roots"),
            ("Vijeto teorema: x₁ + x₂ = −b/a,  x₁ · x₂ = c/a", "Vieta's formulas: x₁ + x₂ = −b/a,  x₁ · x₂ = c/a"),
            ("(a + b)² = a² + 2ab + b²", "(a + b)² = a² + 2ab + b²"),
            ("(a − b)² = a² − 2ab + b²", "(a − b)² = a² − 2ab + b²"),
            ("a² − b² = (a + b)(a − b)", "a² − b² = (a + b)(a − b)"),
            ("aᵐ · aⁿ = aᵐ⁺ⁿ,   aᵐ / aⁿ = aᵐ⁻ⁿ", "aᵐ · aⁿ = aᵐ⁺ⁿ,   aᵐ / aⁿ = aᵐ⁻ⁿ"),
            ("(aᵐ)ⁿ = aᵐⁿ,   a⁰ = 1,   a⁻ⁿ = 1/aⁿ", "(aᵐ)ⁿ = aᵐⁿ,   a⁰ = 1,   a⁻ⁿ = 1/aⁿ"),
            ("log(xy) = log x + log y", "log(xy) = log x + log y"),
            ("log(x/y) = log x − log y,   log(xⁿ) = n · log x", "log(x/y) = log x − log y,   log(xⁿ) = n · log x"),
            ("Procentai: dalis = visuma × p / 100", "Percentages: part = whole × p / 100"),
            ("Aritmetinė progresija: aₙ = a₁ + (n−1)d", "Arithmetic sequence: aₙ = a₁ + (n−1)d"),
            ("Suma: Sₙ = (a₁ + aₙ) · n / 2", "Sum: Sₙ = (a₁ + aₙ) · n / 2"),
            ("Geometrinė progresija: aₙ = a₁ · qⁿ⁻¹", "Geometric sequence: aₙ = a₁ · qⁿ⁻¹"),
        ],
    },
    Section {
        title_lt: "Geometrija",
        title_en: "Geometry",
        lines: &[
            ("Pitagoro teorema: c² = a² + b²", "Pythagoras: c² = a² + b²"),
            ("Trikampio plotas: S = a·h / 2", "Triangle area: S = a·h / 2"),
            ("Herono formulė: S = √(p(p−a)(p−b)(p−c)), p = (a+b+c)/2", "Heron's formula: S = √(p(p−a)(p−b)(p−c)), p = (a+b+c)/2"),
            ("Stačiakampis: S = ab,  P = 2(a + b)", "Rectangle: S = ab,  P = 2(a + b)"),
            ("Kvadratas: S = a²,  įstrižainė = a√2", "Square: S = a²,  diagonal = a√2"),
            ("Lygiagretainis: S = a·h", "Parallelogram: S = a·h"),
            ("Trapecija: S = (a + b)·h / 2", "Trapezoid: S = (a + b)·h / 2"),
            ("Apskritimas: C = 2πr,  S = πr²", "Circle: C = 2πr,  S = πr²"),
            ("Skritulio išpjova: S = πr² · α/360°", "Sector: S = πr² · α/360°"),
            ("Kubas: V = a³,  S = 6a²", "Cube: V = a³,  S = 6a²"),
            ("Gretasienis: V = abc", "Cuboid: V = abc"),
            ("Cilindras: V = πr²h,  S = 2πr² + 2πrh", "Cylinder: V = πr²h,  S = 2πr² + 2πrh"),
            ("Kūgis: V = πr²h / 3", "Cone: V = πr²h / 3"),
            ("Piramidė: V = Sₚ·h / 3", "Pyramid: V = S_base·h / 3"),
            ("Rutulys: V = 4πr³ / 3,  S = 4πr²", "Sphere: V = 4πr³ / 3,  S = 4πr²"),
            ("Trikampio kampų suma: 180°", "Triangle angle sum: 180°"),
        ],
    },
    Section {
        title_lt: "Trigonometrija",
        title_en: "Trigonometry",
        lines: &[
            ("sin²α + cos²α = 1", "sin²α + cos²α = 1"),
            ("tg α = sin α / cos α", "tan α = sin α / cos α"),
            ("sin(α ± β) = sin α cos β ± cos α sin β", "sin(α ± β) = sin α cos β ± cos α sin β"),
            ("cos(α ± β) = cos α cos β ∓ sin α sin β", "cos(α ± β) = cos α cos β ∓ sin α sin β"),
            ("sin 2α = 2 sin α cos α", "sin 2α = 2 sin α cos α"),
            ("cos 2α = cos²α − sin²α", "cos 2α = cos²α − sin²α"),
            ("Sinusų teorema: a/sin A = b/sin B = c/sin C = 2R", "Law of sines: a/sin A = b/sin B = c/sin C = 2R"),
            ("Kosinusų teorema: c² = a² + b² − 2ab·cos C", "Law of cosines: c² = a² + b² − 2ab·cos C"),
            ("sin 30° = 1/2,  sin 45° = √2/2,  sin 60° = √3/2", "sin 30° = 1/2,  sin 45° = √2/2,  sin 60° = √3/2"),
            ("cos 30° = √3/2,  cos 45° = √2/2,  cos 60° = 1/2", "cos 30° = √3/2,  cos 45° = √2/2,  cos 60° = 1/2"),
            ("tg 30° = √3/3,  tg 45° = 1,  tg 60° = √3", "tan 30° = √3/3,  tan 45° = 1,  tan 60° = √3"),
            ("π rad = 180°,  1° = π/180 rad", "π rad = 180°,  1° = π/180 rad"),
        ],
    },
    Section {
        title_lt: "Fizika",
        title_en: "Physics",
        lines: &[
            ("Greitis: v = s / t", "Speed: v = s / t"),
            ("Pagreitis: a = (v − v₀) / t", "Acceleration: a = (v − v₀) / t"),
            ("Tolygiai kintamas judėjimas: s = v₀t + at²/2", "s = v₀t + at²/2"),
            ("Niutono II dėsnis: F = ma", "Newton's second law: F = ma"),
            ("Svertinė jėga: F = mg,  g ≈ 9,8 m/s²", "Weight: F = mg,  g ≈ 9.8 m/s²"),
            ("Trintis: F = μN", "Friction: F = μN"),
            ("Darbas: A = Fs·cos α", "Work: A = Fs·cos α"),
            ("Galia: P = A / t", "Power: P = A / t"),
            ("Kinetinė energija: E = mv² / 2", "Kinetic energy: E = mv² / 2"),
            ("Potencinė energija: E = mgh", "Potential energy: E = mgh"),
            ("Impulsas: p = mv", "Momentum: p = mv"),
            ("Tankis: ρ = m / V", "Density: ρ = m / V"),
            ("Slėgis: p = F / S;  skystyje: p = ρgh", "Pressure: p = F / S;  in fluids: p = ρgh"),
            ("Omo dėsnis: I = U / R", "Ohm's law: I = U / R"),
            ("Elektros galia: P = UI = I²R", "Electric power: P = UI = I²R"),
            ("Šiluma: Q = cmΔT", "Heat: Q = cmΔT"),
        ],
    },
];

/// Section title in the current UI language.
pub fn title(s: &Section) -> &'static str {
    if crate::i18n::lang() == 1 { s.title_lt } else { s.title_en }
}

pub fn line(l: &(&'static str, &'static str)) -> &'static str {
    if crate::i18n::lang() == 1 { l.0 } else { l.1 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn content_is_bilingual() {
        for s in super::SECTIONS {
            assert!(!s.title_lt.is_empty() && !s.title_en.is_empty());
            for (lt, en) in s.lines {
                assert!(!lt.is_empty() && !en.is_empty());
            }
        }
        assert_eq!(super::SECTIONS.len(), 4);
    }
}
