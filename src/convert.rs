//! Unit and currency conversion.

use serde::{Deserialize, Serialize};

pub struct Category { pub name: &'static str, pub units: &'static [(&'static str, f64)] }

pub const CATEGORIES: &[Category] = &[
    Category { name: "Valiuta", units: &[] }, // filled from ECB rates
    Category { name: "Ilgis", units: &[("mm", 0.001), ("cm", 0.01), ("m", 1.0), ("km", 1000.0), ("in (colis)", 0.0254), ("ft (pėda)", 0.3048), ("yd (jardas)", 0.9144), ("mi (mylia)", 1609.344)] },
    Category { name: "Svoris", units: &[("mg", 1e-6), ("g", 0.001), ("kg", 1.0), ("t", 1000.0), ("oz (uncija)", 0.028349523125), ("lb (svaras)", 0.45359237)] },
    Category { name: "Temperatūra", units: &[("°C", 0.0), ("°F", 0.0), ("K", 0.0)] },
    Category { name: "Tūris", units: &[("ml", 0.001), ("l", 1.0), ("m³", 1000.0), ("US gal", 3.785411784), ("UK gal", 4.54609), ("US puodelis", 0.2365882365), ("US fl oz", 0.0295735295625)] },
    Category { name: "Plotas", units: &[("cm²", 1e-4), ("m²", 1.0), ("a (aras)", 100.0), ("ha", 10000.0), ("km²", 1e6), ("ft²", 0.09290304), ("akras", 4046.8564224)] },
    Category { name: "Greitis", units: &[("m/s", 1.0), ("km/h", 1.0 / 3.6), ("mph", 0.44704), ("mazgas", 1852.0 / 3600.0)] },
];

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
    #[test] fn ecb() {
        let xml = "<Cube time='2026-09-23'><Cube currency='USD' rate='1.1000'/><Cube currency='GBP' rate='0.8500'/></Cube>";
        let r = parse_ecb(xml);
        assert_eq!(r.as_ref().unwrap().date, "2026-09-23");
        assert!((convert(0, 0, 1, 10.0, &r).unwrap() - 11.0).abs() < 1e-9);
        assert!((convert(0, 1, 2, 11.0, &r).unwrap() - 8.5).abs() < 1e-9);
    }
}
