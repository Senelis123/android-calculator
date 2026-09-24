//! Unit and currency conversion.

use serde::{Deserialize, Serialize};

pub struct Category { pub name: &'static str, pub units: &'static [(&'static str, f64)] }

pub const CATEGORIES: &[Category] = &[
    Category { name: "Currency", units: &[] }, // filled from ECB rates
    Category { name: "Length", units: &[("mm", 0.001), ("cm", 0.01), ("m", 1.0), ("km", 1000.0), ("in", 0.0254), ("ft", 0.3048), ("yd", 0.9144), ("mi", 1609.344)] },
    Category { name: "Weight", units: &[("mg", 1e-6), ("g", 0.001), ("kg", 1.0), ("t", 1000.0), ("oz", 0.028349523125), ("lb", 0.45359237)] },
    Category { name: "Temperature", units: &[("°C", 0.0), ("°F", 0.0), ("K", 0.0)] },
    Category { name: "Volume", units: &[("ml", 0.001), ("l", 1.0), ("m³", 1000.0), ("US gal", 3.785411784), ("UK gal", 4.54609), ("US cup", 0.2365882365), ("US fl oz", 0.0295735295625)] },
    Category { name: "Area", units: &[("cm²", 1e-4), ("m²", 1.0), ("a", 100.0), ("ha", 10000.0), ("km²", 1e6), ("ft²", 0.09290304), ("acre", 4046.8564224)] },
    Category { name: "Speed", units: &[("m/s", 1.0), ("km/h", 1.0 / 3.6), ("mph", 0.44704), ("kn", 1852.0 / 3600.0)] },
    Category { name: "Pressure", units: &[("Pa", 1.0), ("kPa", 1000.0), ("bar", 1e5), ("atm", 101325.0), ("mmHg", 133.322387415), ("psi", 6894.757293168)] },
    Category { name: "Energy", units: &[("J", 1.0), ("kJ", 1000.0), ("cal", 4.184), ("kcal", 4184.0), ("Wh", 3600.0), ("kWh", 3.6e6)] },
    Category { name: "Power", units: &[("W", 1.0), ("kW", 1000.0), ("hp (metric)", 735.49875), ("hp (US)", 745.69987158227)] },
    Category { name: "Data", units: &[("bit", 0.125), ("B", 1.0), ("KB", 1e3), ("MB", 1e6), ("GB", 1e9), ("TB", 1e12), ("KiB", 1024.0), ("MiB", 1048576.0), ("GiB", 1073741824.0)] },
    Category { name: "Time", units: &[("s", 1.0), ("min", 60.0), ("h", 3600.0), ("day", 86400.0), ("week", 604800.0), ("year", 31557600.0)] },
];

/// Default (from, to) unit per category.
pub const DEFAULTS: &[(usize, usize)] = &[(0, 1), (3, 7), (2, 5), (0, 1), (1, 3), (1, 3), (1, 2), (2, 5), (3, 4), (1, 2), (4, 3), (1, 2)];

/// Parses the ECB 90-day history XML into (date, rates) points for one currency pair (oldest first).
pub fn parse_history(xml: &str, from: &str, to: &str) -> Vec<(String, f64)> {
    let mut out = vec![];
    for day in xml.split("<Cube time=").skip(1) {
        let date = day.trim_start_matches(['\'', '"']).chars().take(10).collect::<String>();
        let rate = |c: &str| -> Option<f64> {
            if c == "EUR" { return Some(1.0); }
            let pos = day.find(&format!("currency='{c}'")).or_else(|| day.find(&format!("currency=\"{c}\"")))?;
            let rest = &day[pos..];
            let r = rest.split("rate=").nth(1)?.trim_start_matches(['\'', '"']);
            r.split(['\'', '"']).next()?.parse().ok()
        };
        if let (Some(a), Some(b)) = (rate(from), rate(to)) { out.push((date, b / a)); }
    }
    out.reverse();
    out
}

pub const CURRENCY_CAT: usize = 0;
pub const TEMP_CAT: usize = 3;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Rates { pub date: String, pub rates: Vec<(String, f64)> } // 1 EUR = rate × currency; EUR included

pub fn parse_ecb(xml: &str) -> Option<Rates> {
    let date = xml.split("time='").nth(1)?.split('\'').next()?.to_string();
    let mut rates = vec![("EUR".to_string(), 1.0)];
    for part in xml.split("currency='").skip(1) {
        let cur = part.split('\'').next()?.to_string();
        let rate: f64 = part.split("rate='").nth(1)?.split('\'').next()?.parse().ok()?;
        rates.push((cur, rate));
    }
    if rates.len() < 2 { return None; }
    Some(Rates { date, rates })
}

pub fn unit_names(cat: usize, rates: &Option<Rates>) -> Vec<String> {
    if cat == CURRENCY_CAT {
        return rates.as_ref().map(|r| r.rates.iter().map(|(c, _)| c.clone()).collect()).unwrap_or_else(|| vec!["EUR".into()]);
    }
    CATEGORIES[cat].units.iter().map(|(n, _)| n.to_string()).collect()
}

pub fn convert(cat: usize, from: usize, to: usize, v: f64, rates: &Option<Rates>) -> Option<f64> {
    if cat == CURRENCY_CAT {
        let r = rates.as_ref()?;
        let (a, b) = (r.rates.get(from)?.1, r.rates.get(to)?.1);
        return Some(v / a * b);
    }
    if cat == TEMP_CAT {
        let c = match from { 0 => v, 1 => (v - 32.0) * 5.0 / 9.0, _ => v - 273.15 };
        return Some(match to { 0 => c, 1 => c * 9.0 / 5.0 + 32.0, _ => c + 273.15 });
    }
    let u = CATEGORIES[cat].units;
    Some(v * u.get(from)?.1 / u.get(to)?.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn units() {
        assert!((convert(1, 3, 2, 1.5, &None).unwrap() - 1500.0).abs() < 1e-9);
        assert!((convert(3, 0, 1, 100.0, &None).unwrap() - 212.0).abs() < 1e-9);
        assert!((convert(3, 2, 0, 0.0, &None).unwrap() + 273.15).abs() < 1e-9);
        assert!((convert(6, 1, 0, 36.0, &None).unwrap() - 10.0).abs() < 1e-9);
    }
    #[test] fn history() {
        let xml = "<Cube><Cube time='2026-09-24'><Cube currency='USD' rate='1.2'/></Cube><Cube time='2026-09-23'><Cube currency='USD' rate='1.1'/></Cube></Cube>";
        let h = parse_history(xml, "EUR", "USD");
        assert_eq!(h, vec![("2026-09-23".to_string(), 1.1), ("2026-09-24".to_string(), 1.2)]);
        assert!((convert(10, 4, 3, 1.0, &None).unwrap() - 1000.0).abs() < 1e-9);
    }
    #[test] fn ecb() {
        let xml = "<Cube time='2026-09-23'><Cube currency='USD' rate='1.1000'/><Cube currency='GBP' rate='0.8500'/></Cube>";
        let r = parse_ecb(xml);
        assert_eq!(r.as_ref().unwrap().date, "2026-09-23");
        assert!((convert(0, 0, 1, 10.0, &r).unwrap() - 11.0).abs() < 1e-9);
        assert!((convert(0, 1, 2, 11.0, &r).unwrap() - 8.5).abs() < 1e-9);
    }
}
