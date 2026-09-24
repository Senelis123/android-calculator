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
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub currency: String,
}

pub const CRYPTOS: &[(&str, &str, &str)] = &[
    ("bitcoin", "BTC", "Bitcoin"),
    ("ethereum", "ETH", "Ethereum"),
    ("solana", "SOL", "Solana"),
    ("binancecoin", "BNB", "BNB"),
    ("ripple", "XRP", "XRP"),
    ("dogecoin", "DOGE", "Dogecoin"),
];

pub const STOCKS: &[(&str, &str, &str)] = &[
    ("AAPL", "AAPL", "Apple"),
    ("MSFT", "MSFT", "Microsoft"),
    ("NVDA", "NVDA", "NVIDIA"),
    ("GOOGL", "GOOGL", "Alphabet"),
    ("AMZN", "AMZN", "Amazon"),
    ("TSLA", "TSLA", "Tesla"),
];

pub fn crypto_url() -> String {
    let ids = CRYPTOS.iter().map(|(id, _, _)| *id).collect::<Vec<_>>().join(",");
    format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={ids}&vs_currencies=usd,eur&include_24hr_change=true"
    )
}

pub fn stock_url(symbol: &str) -> String {
    format!("https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range=1d&interval=1d")
}

pub fn parse_crypto(json: &str) -> Vec<Quote> {
    let Ok(root): Result<Value, _> = serde_json::from_str(json) else { return vec![] };
    let Some(obj) = root.as_object() else { return vec![] };
    let mut out = Vec::with_capacity(CRYPTOS.len());
    for (id, symbol, name) in CRYPTOS {
        let Some(q) = obj.get(*id).and_then(Value::as_object) else { continue };
        let price = q.get("eur").and_then(Value::as_f64)
            .or_else(|| q.get("usd").and_then(Value::as_f64));
        let change = q.get("eur_24h_change").and_then(Value::as_f64)
            .or_else(|| q.get("usd_24h_change").and_then(Value::as_f64));
        if let Some(price) = price {
            out.push(Quote {
                symbol: (*symbol).into(),
                name: (*name).into(),
                price,
                change_pct: change,
                currency: "EUR".into(),
            });
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct YahooChart { chart: YahooChartInner }
#[derive(Debug, Deserialize)]
struct YahooChartInner { result: Option<Vec<YahooResult>> }
#[derive(Debug, Deserialize)]
struct YahooResult { meta: YahooMeta }
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
    let root: YahooChart = serde_json::from_str(json).ok()?;
    let meta = root.chart.result?.into_iter().next()?.meta;
    let price = meta.regular_market_price?;
    let change_pct = meta.chart_previous_close.and_then(|prev| {
        (prev != 0.0).then_some((price / prev - 1.0) * 100.0)
    });
    let symbol = meta.symbol.unwrap_or_else(|| fallback_symbol.to_string());
    let name = meta.short_name.unwrap_or_else(|| symbol.clone());
    Some(Quote {
        symbol,
        name,
        price,
        change_pct,
        currency: meta.currency.unwrap_or_else(|| "USD".into()),
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
    fn yahoo_parser() {
        let j = r#"{"chart":{"result":[{"meta":{"regularMarketPrice":200.0,"chartPreviousClose":190.0,"currency":"USD","symbol":"AAPL","shortName":"Apple Inc."}}]}}"#;
        let q = parse_stock(j, "AAPL").unwrap();
        assert_eq!(q.symbol, "AAPL");
        assert_eq!(q.name, "Apple Inc.");
        assert!((q.change_pct.unwrap() - 5.2631578947).abs() < 1e-9);
    }
}
