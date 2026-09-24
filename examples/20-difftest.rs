use std::io::BufRead;
fn main() {
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut c = calculator::calc::Calculator::new();
        let mut out = String::new();
        for k in line.split(' ').filter(|k| !k.is_empty()) { c.press(k); out.push_str(c.display()); out.push('|'); }
        println!("{}", out);
    }
}
