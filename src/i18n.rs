//! Translations. English is the default language; others come from i18n/strings.tsv.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

#[path = "i18n_table.rs"]
mod table;
pub use table::LANGS;

/// Native names shown in Settings, same order as LANGS.
pub const LANG_NAMES: &[&str] = &["English", "Lietuvių", "Latviešu", "Polski", "Deutsch", "Русский", "Українська"];

static LANG: AtomicUsize = AtomicUsize::new(0);

pub fn set_lang(i: usize) { LANG.store(i.min(LANGS.len() - 1), Ordering::Relaxed); }
pub fn lang() -> usize { LANG.load(Ordering::Relaxed) }
/// Index of a language code like "lt" or "lt-LT"; English when unknown.
pub fn lang_from_code(code: &str) -> usize {
    let c = code.split(['-', '_']).next().unwrap_or("").to_lowercase();
    LANGS.iter().position(|l| *l == c).unwrap_or(0)
}

fn index() -> &'static HashMap<&'static str, usize> {
    static IDX: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    IDX.get_or_init(|| table::TABLE.iter().enumerate().map(|(i, r)| (r[0], i)).collect())
}

/// Translates an English UI string (falls back to English).
pub fn t(en: &'static str) -> &'static str {
    let l = lang();
    if l == 0 { return en; }
    match index().get(en) { Some(&i) => { let s = table::TABLE[i][l]; if s.is_empty() { en } else { s } } None => en }
}

/// Translates and fills `{}` placeholders in order.
pub fn tf(en: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut it = args.iter();
    let mut parts = t(en).split("{}").peekable();
    while let Some(p) = parts.next() {
        out.push_str(p);
        if parts.peek().is_some() { if let Some(a) = it.next() { out.push_str(&a.to_string()); } }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn works() {
        assert_eq!(t("History"), "History");
        set_lang(1); assert_eq!(t("History"), "Istorija");
        assert_eq!(tf("VAT {} %", &[&21]), "PVM 21 %");
        set_lang(0);
        assert_eq!(lang_from_code("lt-LT"), 1); assert_eq!(lang_from_code("fr"), 0);
        for r in table::TABLE.iter() { for c in r.iter() { assert_eq!(c.matches("{}").count(), r[0].matches("{}").count(), "{}", r[0]); } }
    }
}
