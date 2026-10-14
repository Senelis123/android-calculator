//! Informational stock and cryptocurrency quotes.
//!
//! The screen only displays market data. It has no trading actions.
//! Crypto quotes use CoinGecko's public simple-price endpoint;
//! stock quotes use Yahoo Finance chart quote metadata.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum MarketKind {
    Crypto,
    Stocks,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    /// Provider id: CoinGecko id for crypto, ticker for stocks (used for the history chart).
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub currency: String,
    pub info: Vec<(String, String)>,
}

pub const CRYPTOS: &[(&str, &str, &str)] = &[
    ("bitcoin", "BTC", "Bitcoin"),
    ("ethereum", "ETH", "Ethereum"),
    ("solana", "SOL", "Solana"),
    ("binancecoin", "BNB", "BNB"),
    ("ripple", "XRP", "XRP"),
    ("dogecoin", "DOGE", "Dogecoin"),
    ("cardano", "ADA", "Cardano"),
    ("tron", "TRX", "TRON"),
    ("avalanche-2", "AVAX", "Avalanche"),
    ("chainlink", "LINK", "Chainlink"),
    ("litecoin", "LTC", "Litecoin"),
    ("polkadot", "DOT", "Polkadot"),
];

pub const STOCKS: &[(&str, &str, &str)] = &[
    ("AAPL", "AAPL", "Apple"),
    ("MSFT", "MSFT", "Microsoft"),
    ("NVDA", "NVDA", "NVIDIA"),
    ("GOOGL", "GOOGL", "Alphabet"),
    ("AMZN", "AMZN", "Amazon"),
    ("TSLA", "TSLA", "Tesla"),
    ("META", "META", "Meta"),
    ("NFLX", "NFLX", "Netflix"),
    ("AMD", "AMD", "AMD"),
    ("INTC", "INTC", "Intel"),
    ("DIS", "DIS", "Disney"),
    ("KO", "KO", "Coca-Cola"),
    ("MCD", "MCD", "McDonald's"),
    ("NKE", "NKE", "Nike"),
];

/// History ranges shown as chips on the detail screen: (label, CoinGecko days, Yahoo range, Yahoo interval).
pub const RANGES: &[(&str, &str, &str, &str)] = &[
    ("1D", "1", "1d", "15m"),
    ("7D", "7", "5d", "1h"),
    ("1M", "30", "1mo", "1d"),
    ("3M", "90", "3mo", "1d"),
    ("1Y", "365", "1y", "1d"),
];

pub fn history_url(kind: MarketKind, id: &str, range: usize) -> String {
    let (_, days, yr, yi) = RANGES[range.min(RANGES.len() - 1)];
    match kind {
        MarketKind::Crypto => format!(
            "https://api.coingecko.com/api/v3/coins/{id}/market_chart?vs_currency=eur&days={days}"
        ),
        MarketKind::Stocks => format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{id}?range={yr}&interval={yi}"
        ),
    }
}

pub fn history_backup_url(kind: MarketKind, id: &str, range: usize) -> String {
    let (_, _, yr, yi) = RANGES[range.min(RANGES.len() - 1)];
    match kind {
        MarketKind::Crypto => history_url(kind, id, range),
        MarketKind::Stocks => {
            format!(
                "https://query2.finance.yahoo.com/v8/finance/chart/{id}?range={yr}&interval={yi}"
            )
        }
    }
}

/// Price points (unix seconds, price), oldest first. Empty when the reply is not understood.
pub fn parse_history(kind: MarketKind, json: &str) -> Vec<(f64, f64)> {
    let Ok(root): Result<Value, _> = serde_json::from_str(json) else {
        return vec![];
    };
    let mut out = vec![];
    match kind {
        MarketKind::Crypto => {
            if let Some(a) = root.get("prices").and_then(Value::as_array) {
                for p in a {
                    let (t, v) = (
                        p.get(0).and_then(Value::as_f64),
                        p.get(1).and_then(Value::as_f64),
                    );
                    if let (Some(t), Some(v)) = (t, v) {
                        if v.is_finite() {
                            out.push((t / 1000.0, v));
                        }
                    }
                }
            }
        }
        MarketKind::Stocks => {
            let r = root.pointer("/chart/result/0");
            let ts = r.and_then(|r| r.get("timestamp")).and_then(Value::as_array);
            let cl = r
                .and_then(|r| r.pointer("/indicators/quote/0/close"))
                .and_then(Value::as_array);
            if let (Some(ts), Some(cl)) = (ts, cl) {
                for (t, c) in ts.iter().zip(cl.iter()) {
                    if let (Some(t), Some(c)) = (t.as_f64(), c.as_f64()) {
                        if c.is_finite() {
                            out.push((t, c));
                        }
                    }
                }
            }
        }
    }
    out
}

pub fn crypto_url() -> String {
    let ids = CRYPTOS
        .iter()
        .map(|(id, _, _)| *id)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={ids}&vs_currencies=usd,eur&include_24hr_change=true&include_market_cap=true&include_24hr_vol=true&include_last_updated_at=true"
    )
}

pub fn stock_url(symbol: &str) -> String {
    format!("https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range=1d&interval=1d")
}

pub fn stock_backup_url(symbol: &str) -> String {
    format!("https://query2.finance.yahoo.com/v8/finance/chart/{symbol}?range=1d&interval=1d")
}

pub fn parse_crypto(json: &str) -> Vec<Quote> {
    let Ok(root): Result<Value, _> = serde_json::from_str(json) else {
        return vec![];
    };
    let Some(obj) = root.as_object() else {
        return vec![];
    };
    let mut out = Vec::with_capacity(CRYPTOS.len());
    for (id, symbol, name) in CRYPTOS {
        let Some(q) = obj.get(*id).and_then(Value::as_object) else {
            continue;
        };
        let eur = q
            .get("eur")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0);
        let usd = q
            .get("usd")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0);
        let price = eur.or(usd);
        let change = q
            .get(if eur.is_some() {
                "eur_24h_change"
            } else {
                "usd_24h_change"
            })
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite());
        if let Some(price) = price {
            out.push(Quote {
                id: (*id).into(),
                symbol: (*symbol).into(),
                name: (*name).into(),
                price,
                change_pct: change,
                currency: if eur.is_some() { "EUR" } else { "USD" }.into(),
                info: {
                    let pre = if eur.is_some() { "eur" } else { "usd" };
                    let mut a = vec![];
                    for (label, key) in [
                        ("Market cap", format!("{pre}_market_cap")),
                        ("24h volume", format!("{pre}_24h_vol")),
                    ] {
                        if let Some(v) = q
                            .get(&key)
                            .and_then(Value::as_f64)
                            .filter(|v| v.is_finite())
                        {
                            a.push((label.into(), crate::expr::display_num(v)));
                        }
                    }
                    if let Some(v) = q.get("last_updated_at").and_then(Value::as_f64) {
                        a.push(("Updated (UTC)".into(), crate::app::fmt_ts(v, true)));
                    }
                    a
                },
            });
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct YahooChart {
    chart: YahooChartInner,
}
#[derive(Debug, Deserialize)]
struct YahooChartInner {
    result: Option<Vec<YahooResult>>,
}
#[derive(Debug, Deserialize)]
struct YahooResult {
    meta: YahooMeta,
}
#[derive(Debug, Deserialize)]
struct YahooMeta {
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "chartPreviousClose")]
    chart_previous_close: Option<f64>,
    currency: Option<String>,
    symbol: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
}

pub fn parse_stock(json: &str, fallback_symbol: &str) -> Option<Quote> {
    let raw: Value = serde_json::from_str(json).ok()?;
    let root: YahooChart = serde_json::from_str(json).ok()?;
    let meta = root.chart.result?.into_iter().next()?.meta;
    let price = meta
        .regular_market_price
        .filter(|v| v.is_finite() && *v >= 0.0)?;
    let change_pct = meta
        .chart_previous_close
        .and_then(|prev| (prev.is_finite() && prev > 0.0).then_some((price / prev - 1.0) * 100.0));
    let symbol = meta.symbol.unwrap_or_else(|| fallback_symbol.to_string());
    let name = meta.short_name.unwrap_or_else(|| symbol.clone());
    Some(Quote {
        id: fallback_symbol.to_string(),
        symbol,
        name,
        price,
        change_pct,
        currency: meta.currency.unwrap_or_else(|| "USD".into()),
        info: {
            let mut a = vec![];
            if let Some(m) = raw.pointer("/chart/result/0/meta") {
                for (label, key) in [
                    ("Day high", "regularMarketDayHigh"),
                    ("Day low", "regularMarketDayLow"),
                    ("Volume", "regularMarketVolume"),
                    ("Previous close", "chartPreviousClose"),
                    ("52 week high", "fiftyTwoWeekHigh"),
                    ("52 week low", "fiftyTwoWeekLow"),
                    ("Market cap", "marketCap"),
                ] {
                    if let Some(v) = m.get(key).and_then(Value::as_f64).filter(|v| v.is_finite()) {
                        a.push((label.into(), crate::expr::display_num(v)));
                    }
                }
                for (label, key) in [
                    ("Exchange", "exchangeName"),
                    ("Instrument", "instrumentType"),
                ] {
                    if let Some(v) = m.get(key).and_then(Value::as_str) {
                        a.push((label.into(), v.into()));
                    }
                }
                if let Some(v) = m.get("regularMarketTime").and_then(Value::as_f64) {
                    a.push(("Updated (UTC)".into(), crate::app::fmt_ts(v, true)));
                }
            }
            a
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crypto_parser() {
        let j = r#"{"bitcoin":{"eur":55000.0,"eur_24h_change":1.25}}"#;
        let q = parse_crypto(j);
        assert_eq!(q[0].symbol, "BTC");
        assert_eq!(q[0].price, 55000.0);
        assert_eq!(q[0].change_pct, Some(1.25));
    }
    #[test]
    fn crypto_usd_fallback_is_labeled_usd() {
        let q = parse_crypto(r#"{"bitcoin":{"usd":85000.0,"usd_24h_change":2.0}}"#);
        assert_eq!(q[0].currency, "USD");
        assert_eq!(q[0].change_pct, Some(2.0));
    }
    #[test]
    fn invalid_stock_price_is_not_shown() {
        let j = r#"{"chart":{"result":[{"meta":{"regularMarketPrice":-1,"currency":"USD"}}]}}"#;
        assert!(parse_stock(j, "AAPL").is_none());
    }
    #[test]
    fn history_parsers() {
        let c = parse_history(
            MarketKind::Crypto,
            r#"{"prices":[[1000000,5.0],[2000000,6.5]]}"#,
        );
        assert_eq!(c, vec![(1000.0, 5.0), (2000.0, 6.5)]);
        let y = r#"{"chart":{"result":[{"timestamp":[1,2,3],"indicators":{"quote":[{"close":[1.0,null,3.0]}]}}]}}"#;
        assert_eq!(
            parse_history(MarketKind::Stocks, y),
            vec![(1.0, 1.0), (3.0, 3.0)]
        );
    }
    #[test]
    fn yahoo_parser() {
        let j = r#"{"chart":{"result":[{"meta":{"regularMarketPrice":200.0,"chartPreviousClose":190.0,"currency":"USD","symbol":"AAPL","shortName":"Apple Inc."}}]}}"#;
        let q = parse_stock(j, "AAPL").unwrap();
        assert_eq!(q.symbol, "AAPL");
        assert_eq!(q.name, "Apple Inc.");
        assert!((q.change_pct.unwrap() - 5.2631578947).abs() < 1e-9);
    }
}
