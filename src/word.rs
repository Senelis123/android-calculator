//! Offline word-problem solver (Lithuanian / English): turns a short story problem into an arithmetic expression.
//! Rule based: it reads the numbers with their units and picks a formula from keywords. Returns None when unsure.

struct Num {
    v: f64,
    unit: String,
    pct: bool,
}

fn word_value(w: &str) -> Option<f64> {
    const W: &[(&str, f64)] = &[
        ("one", 1.0), ("two", 2.0), ("three", 3.0), ("four", 4.0), ("five", 5.0), ("six", 6.0),
        ("seven", 7.0), ("eight", 8.0), ("nine", 9.0), ("ten", 10.0), ("twelve", 12.0), ("twenty", 20.0),
        ("thirty", 30.0), ("forty", 40.0), ("fifty", 50.0), ("hundred", 100.0),
        ("vienas", 1.0), ("viena", 1.0), ("du", 2.0), ("dvi", 2.0), ("trys", 3.0), ("keturi", 4.0),
        ("keturios", 4.0), ("penki", 5.0), ("penkios", 5.0), ("šeši", 6.0), ("šešios", 6.0), ("septyni", 7.0),
        ("septynios", 7.0), ("aštuoni", 8.0), ("aštuonios", 8.0), ("devyni", 9.0), ("devynios", 9.0),
        ("dešimt", 10.0), ("dvylika", 12.0), ("dvidešimt", 20.0), ("trisdešimt", 30.0),
        ("keturiasdešimt", 40.0), ("penkiasdešimt", 50.0), ("šimtas", 100.0), ("pusantro", 1.5),
        ("šimtą", 100.0), ("vieną", 1.0), ("tris", 3.0), ("keturis", 4.0), ("penkis", 5.0), ("šešis", 6.0),
    ];
    W.iter().find(|(k, _)| *k == w).map(|(_, v)| *v)
}

fn clean(w: &str) -> String {
    w.trim_matches(|c: char| !(c.is_alphanumeric() || c == '%' || c == '/' || c == '.' || c == ','))
        .trim_end_matches(['.', ','])
        .to_string()
}

fn parse(text: &str) -> (Vec<Num>, Vec<String>, String) {
    let low = text.to_lowercase();
    let words: Vec<String> = low.split_whitespace().map(clean).filter(|w| !w.is_empty()).collect();
    let mut nums = vec![];
    for (i, w) in words.iter().enumerate() {
        let unit_of = |j: usize| words.get(j).cloned().unwrap_or_default();
        let t = w.replace(',', ".");
        let (body, pct) = match t.strip_suffix('%') {
            Some(b) => (b.to_string(), true),
            None => (t.clone(), false),
        };
        if let Ok(v) = body.parse::<f64>() {
            let pct = pct || matches!(unit_of(i + 1).as_str(), "%" | "proc" | "procentų" | "procentai" | "procentas" | "percent");
            nums.push(Num { v, unit: unit_of(i + 1), pct });
        } else if let Some((a, b)) = body.split_once('/') {
            if let (Ok(a), Ok(b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                if b != 0.0 && !a.to_string().is_empty() && !unit_of(i + 1).starts_with("h") {
                    nums.push(Num { v: a / b, unit: unit_of(i + 1), pct: false });
                }
            }
        } else if let Some(v) = word_value(w) {
            nums.push(Num { v, unit: unit_of(i + 1), pct: false });
        }
    }
    (nums, words, low)
}

fn f(v: f64) -> String {
    let s = format!("{v}");
    if v < 0.0 { format!("({s})") } else { s }
}

fn has(low: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| low.contains(k))
}

fn unit_is(n: &Num, keys: &[&str]) -> bool {
    keys.iter().any(|k| n.unit.starts_with(k))
}

pub fn word_to_expr(text: &str) -> Option<String> {
    let (nums, words, low) = parse(text);
    if nums.is_empty() || words.windows(2).any(|w| word_value(&w[0]).is_some() && word_value(&w[1]).is_some()) {
        return None;
    }
    let plain: Vec<f64> = nums.iter().filter(|n| !n.pct).map(|n| n.v).collect();
    let n = nums.len();
    let asks = |keys: &[&str]| has(&low, keys);

    // what percent is A of B
    if n >= 2 && asks(&["what percent", "what percentage", "kiek procentų", "kiek proc", "kokią dalį procentais"]) {
        return Some(format!("{} / {} * 100", f(nums[0].v), f(nums[1].v)));
    }
    // percent discount / increase of a price
    if let (Some(p), true) = (nums.iter().find(|x| x.pct), !plain.is_empty()) {
        let base = plain[0];
        if asks(&["discount", "off", "sale", "nuolaid", "atpigo", "pigiau", "sumažėj", "sumažin", "decrease", "reduced", "mažiau"]) {
            return Some(format!("{} * (1 - {} / 100)", f(base), f(p.v)));
        }
        if asks(&["increase", "increased", "more", "padidėj", "pabrang", "brangiau", "tax", "pvm", "vat", "tip", "daugiau", "išaugo", "interest", "palūkan"]) {
            return Some(format!("{} * (1 + {} / 100)", f(base), f(p.v)));
        }
        if asks(&["of", "nuo", "from", "iš"]) {
            return Some(format!("{} / 100 * {}", f(p.v), f(base)));
        }
    }
    // geometry
    let shape = |keys: &[&str]| asks(keys);
    let area = asks(&["area", "plot"]);
    let perim = asks(&["perimeter", "perimetr", "around"]);
    if shape(&["rectangle", "stačiakamp", "room", "kambar", "field", "laukas", "lauko"]) && plain.len() >= 2 && (area || perim) {
        return Some(if perim { format!("2 * ({} + {})", f(plain[0]), f(plain[1])) } else { format!("{} * {}", f(plain[0]), f(plain[1])) });
    }
    if shape(&["square", "kvadrat"]) && !plain.is_empty() && (area || perim) && !asks(&["root", "šakn"]) {
        return Some(if perim { format!("4 * {}", f(plain[0])) } else { format!("{} ^ 2", f(plain[0])) });
    }
    if shape(&["triangle", "trikamp"]) && plain.len() >= 2 && area {
        return Some(format!("{} * {} / 2", f(plain[0]), f(plain[1])));
    }
    if shape(&["circle", "apskritim", "skritul", "ratas", "wheel"]) && !plain.is_empty() {
        let r = if asks(&["diameter", "skersmuo"]) { format!("{} / 2", f(plain[0])) } else { f(plain[0]) };
        if perim || asks(&["circumference", "ilgis"]) {
            return Some(format!("2 * 3.14159265358979 * ({r})"));
        }
        if area {
            return Some(format!("3.14159265358979 * ({r}) ^ 2"));
        }
    }
    if shape(&["cube", "kub"]) && !plain.is_empty() && asks(&["volume", "tūris"]) {
        return Some(format!("{} ^ 3", f(plain[0])));
    }
    if asks(&["volume", "tūris"]) && plain.len() >= 3 {
        return Some(format!("{} * {} * {}", f(plain[0]), f(plain[1]), f(plain[2])));
    }
    // speed / time / distance
    let speed = nums.iter().find(|x| unit_is(x, &["km/h", "kmh", "km/val", "m/s", "mph", "kilometrų/val"]) || (unit_is(x, &["km", "kilomet"]) && false));
    let time = nums.iter().find(|x| unit_is(x, &["h", "hour", "val", "min"]) && !unit_is(x, &["km/h"]) && !unit_is(x, &["m/s"]));
    let dist = nums.iter().find(|x| unit_is(x, &["km", "kilomet", "mile", "mylios", "mylių", "metr", "m "]) && !unit_is(x, &["km/h", "km/val"]));
    let wants_dist = asks(&["how far", "distance", "kiek kilometrų", "kiek nuvažiuos", "kokį atstumą", "atstum", "kiek metrų", "how many km", "how many kilometers"]);
    let wants_time = asks(&["how long", "how many hours", "how much time", "kiek laiko", "kiek valandų", "kiek minučių", "per kiek"]);
    let wants_speed = asks(&["how fast", "average speed", "what speed", "kokiu greičiu", "koks greitis", "kokia greitis", "kokį greitį", "greitis"]) && !wants_dist && !wants_time;
    if let (Some(s), Some(t), true) = (speed, time, wants_dist || !(wants_time || wants_speed)) {
        if asks(&["speed", "greit", "km/h"]) {
            let m = if unit_is(t, &["min"]) { " / 60" } else { "" };
            return Some(format!("{} * {}{}", f(s.v), f(t.v), m));
        }
    }
    if let (Some(d), Some(s), true) = (dist, speed, wants_time) {
        return Some(format!("{} / {}", f(d.v), f(s.v)));
    }
    if let (Some(d), Some(t), true) = (dist, time, wants_speed) {
        return Some(format!("{} / {}", f(d.v), f(t.v)));
    }
    // average
    if asks(&["average", "mean", "vidurk", "vidutin"]) && plain.len() >= 2 && !asks(&["speed", "greit"]) {
        let s: Vec<String> = plain.iter().map(|v| f(*v)).collect();
        return Some(format!("({}) / {}", s.join(" + "), plain.len()));
    }
    // fraction words
    let frac = if asks(&["half of", "pusė", "pusę", "pusės"]) { Some(2.0) }
        else if asks(&["third of", "trečdal", "trečdalį"]) { Some(3.0) }
        else if asks(&["quarter of", "ketvirtis", "ketvirtadal", "ketvirtį"]) { Some(4.0) }
        else if asks(&["fifth of", "penktadal"]) { Some(5.0) }
        else { None };
    if let (Some(d), false) = (frac, plain.is_empty()) {
        let k = if words.iter().any(|w| matches!(w.as_str(), "two" | "du" | "dvi")) && plain.len() >= 2 { 1 } else { 0 };
        let _ = k;
        return Some(format!("{} / {}", f(*plain.last().unwrap()), d));
    }
    // "A of B" fractions like 3/4 of 80 (a < 1 parsed from a fraction)
    if plain.len() == 2 && plain[0] > 0.0 && plain[0] < 1.0 && asks(&[" of ", " nuo ", " iš "]) {
        return Some(format!("{} * {}", f(plain[0]), f(plain[1])));
    }
    // N times more / less
    if plain.len() == 2 && asks(&["times more", "times as many", "times as much", "kartų daugiau", "kartus daugiau", "kart daugiau", "kartų didesnis", "kartų brangiau"]) {
        return Some(format!("{} * {}", f(plain[0]), f(plain[1])));
    }
    if plain.len() == 2 && asks(&["times less", "times fewer", "kartų mažiau", "kartus mažiau", "kartų mažesnis", "kartų pigiau"]) {
        return Some(format!("{} / {}", f(plain[0]), f(plain[1])));
    }
    // unit price: a things cost b, how much do c cost
    if plain.len() == 3 && asks(&["cost", "kainuoja", "kainuos", "price", "kaina"]) && asks(&["how much", "how many", "kiek"]) {
        let (a, b, c) = (plain[0], plain[1], plain[2]);
        if a != 0.0 && (asks(&["for", "už", "cost", "kainuoja", "buy", "perka", "nupirk"])) {
            return Some(format!("{} / {} * {}", f(b), f(a), f(c)));
        }
    }
    // equal sharing
    if plain.len() >= 2 && asks(&["equally", "po lygiai", "dalij", "share", "split", "each get", "kiekvienas gaus", "kiekvienam", "per person", "vienam", "on each", "kiekvienoje", "each box", "each bag", "into groups"])
        && !asks(&["each costs", "kainuoja po"])
        && (!asks(&["each cost", "po ", "at "]) || asks(&["share", "split", "equally", "po lygiai", "dalij"]))
    {
        return Some(format!("{} / {}", f(plain[0]), f(plain[1])));
    }
    // each / per : price * count
    if plain.len() == 2 && asks(&["each", "per ", "every", "kiekvien", " po ", "kaina", "kainuoja", "kainuos"]) && asks(&["total", "altogether", "in all", "iš viso", "kiek kainuos", "how much", "kiek sumokė", "kiek reikės", "kiek", "how many"]) {
        return Some(format!("{} * {}", f(plain[0]), f(plain[1])));
    }
    if asks(&["product", "sandauga", "multiply", "padaugin"]) && plain.len() >= 2 {
        let s: Vec<String> = plain.iter().map(|v| f(*v)).collect();
        return Some(s.join(" * "));
    }
    if asks(&["quotient", "dalmuo", "divide", "padalin"]) && plain.len() >= 2 {
        return Some(format!("{} / {}", f(plain[0]), f(plain[1])));
    }
    if asks(&["difference", "skirtumas", "how many more", "kiek daugiau", "kiek mažiau", "how many fewer", "how much more", "how much less", "kiek brangiau", "kiek pigiau", "kiek labiau"]) && plain.len() >= 2 {
        let (a, b) = (plain[0].max(plain[1]), plain[0].min(plain[1]));
        return Some(format!("{} - {}", f(a), f(b)));
    }
    if asks(&["sum of", "suma", "total", "altogether", "in all", "iš viso", "kartu", "together", "combined", "bendrai", "sudėk", "add up", "in total"]) && plain.len() >= 2
        && !asks(&["left", "remain", "liko", "liks", "spent", "išleido"])
    {
        let s: Vec<String> = plain.iter().map(|v| f(*v)).collect();
        return Some(s.join(" + "));
    }
    // running story: start with first number, add or subtract by verbs per clause
    if plain.len() >= 2 {
        let lose = ["gave", "lost", "ate", "sold", "spent", "used", "left", "gives", "loses", "eats", "sells", "spends", "atidav", "prarado", "suvalg", "pardav", "išleido", "išnaud", "padav", "sumok", "sugedo", "liko", "nukrito", "išėjo", "išvyko", "atėm", "išgėr", "sunaudo", "dovanoj", "paskolino", "left", "took away", "broke", "removed", "pašalin"];
        let gain = ["bought", "got", "found", "received", "added", "earned", "buys", "gets", "finds", "pirko", "gavo", "rado", "nupirko", "pridėjo", "uždirbo", "atnešė", "atvyko", "įdėjo", "gauna", "more", "daugiau", "padovanojo", "laimėjo", "surinko", "prisidėjo", "joined", "came"];
        let norm = low.replace([',', ';'], ".").replace(" and ", ". ").replace(" ir ", ". ").replace(" but ", ". ").replace(" bet ", ". ").replace(" then ", ". ").replace(" tada ", ". ");
        let mut e = String::new();
        let mut first = true;
        let mut seen_any = false;
        for clause in norm.split('.') {
            let (cn, _, cl) = parse(clause);
            let cnums: Vec<&Num> = cn.iter().filter(|x| !x.pct).collect();
            if cnums.is_empty() {
                continue;
            }
            let v = cnums[0].v;
            if first {
                e = f(v);
                first = false;
                continue;
            }
            if has(&cl, &lose) {
                e = format!("{e} - {}", f(v));
                seen_any = true;
            } else if has(&cl, &gain) {
                e = format!("{e} + {}", f(v));
                seen_any = true;
            } else if !seen_any {
                e = format!("{e} + {}", f(v));
            } else {
                return None;
            }
        }
        if seen_any {
            return Some(e);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::eval_text;
    fn ans(t: &str) -> f64 {
        let e = word_to_expr(t).unwrap_or_else(|| panic!("no expr for {t}"));
        eval_text(&e).unwrap_or_else(|| panic!("bad expr {e}"))
    }
    #[test]
    fn english() {
        assert_eq!(ans("A jacket costs 80 euros with a 25% discount. What is the new price?"), 60.0);
        assert_eq!(ans("A car drives at 60 km/h for 3 hours. How far does it go?"), 180.0);
        assert_eq!(ans("The average of 4, 8 and 12"), 8.0);
        assert_eq!(ans("Tom has 15 apples, gave 6 to Ann and bought 4 more. How many does he have?"), 13.0);
        assert_eq!(ans("A rectangle is 7 m by 5 m. What is its area?"), 35.0);
        assert_eq!(ans("What percent is 15 of 60?"), 25.0);
        assert_eq!(ans("3 books cost 36 euros. How much do 5 books cost?"), 60.0);
        assert_eq!(ans("What is half of 90?"), 45.0);
    }
    #[test]
    fn lithuanian() {
        assert_eq!(ans("Striukė kainuoja 80 eurų, nuolaida 25%. Kiek ji kainuoja dabar?"), 60.0);
        assert_eq!(ans("Automobilis važiuoja 60 km/h greičiu 3 valandas. Kiek kilometrų nuvažiuos?"), 180.0);
        assert_eq!(ans("Jonas turėjo 15 obuolių, atidavė 6 ir nupirko 4. Kiek obuolių turi?"), 13.0);
        assert_eq!(ans("Stačiakampio kraštinės 7 ir 5. Koks plotas?"), 35.0);
        assert_eq!(ans("Kiek procentų 15 sudaro nuo 60?"), 25.0);
        assert_eq!(ans("3 sąsiuviniai kainuoja 6 eurus. Kiek kainuos 5 sąsiuviniai?"), 10.0);
        assert_eq!(ans("Vidurkis 4, 8 ir 12"), 8.0);
        assert_eq!(ans("Kelias 120 km, greitis 60 km/h. Kiek laiko užtruks?"), 2.0);
    }
}
