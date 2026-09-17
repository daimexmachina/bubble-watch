//! Yahoo Finance chart client.
//!
//! Endpoint shape (verified against the live host):
//!   /v8/finance/chart/{SYMBOL}?range=1y&interval=1d
//!   -> chart.result[0].timestamp[]
//!      chart.result[0].indicators.quote[0].close[]
//!      chart.result[0].indicators.adjclose[0].adjclose[]
//!
//! Yahoo's API is unofficial and can rate-limit without notice, so every call
//! goes through the shared `Fetcher` (throttle + retry). A failure produces a
//! `SourceFailure` and therefore a coverage gap, never a zero.

use crate::http::{Fetcher, UA_WEB};
use crate::model::{Point, Provenance, Series};

pub fn chart(f: &Fetcher, symbol: &str, range: &str, interval: &str) -> Result<Series, String> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
        symbol, range, interval
    );
    let body = f.get(&url, UA_WEB)?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("bad JSON from {}: {}", url, e))?;

    let result = v
        .get("chart")
        .and_then(|c| c.get("result"))
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| format!("no chart.result[] for {}", symbol))?;

    let ts = result
        .get("timestamp")
        .and_then(|t| t.as_array())
        .ok_or_else(|| format!("no timestamp[] for {}", symbol))?;

    let closes = result
        .get("indicators")
        .and_then(|i| i.get("adjclose").or_else(|| i.get("quote")))
        .and_then(|q| q.as_array())
        .and_then(|a| a.first())
        .and_then(|q| {
            q.get("adjclose")
                .and_then(|a| a.as_array())
                .or_else(|| q.get("close").and_then(|c| c.as_array()))
        })
        .ok_or_else(|| format!("no close[] for {}", symbol))?;

    let mut points = Vec::new();
    for (i, t) in ts.iter().enumerate() {
        let Some(v) = closes.get(i).and_then(|c| c.as_f64()) else {
            continue; // Yahoo emits nulls for non-trading gaps
        };
        let Some(epoch) = t.as_i64() else { continue };
        points.push(Point {
            date: epoch_to_date(epoch),
            value: v,
        });
    }

    if points.is_empty() {
        return Err(format!("no usable close points for {}", symbol));
    }

    let as_of = points.last().map(|p| p.date.clone()).unwrap_or_default();
    Ok(Series {
        provenance: Provenance {
            source: "yahoo".into(),
            endpoint: url,
            as_of,
            retrieved_at: crate::now_iso8601(),
        },
        points,
    })
}

/// Format a Unix epoch as YYYY-MM-DD in UTC without pulling in a chrono type
/// conversion (keeps the pure core free of time-zone assumptions).
fn epoch_to_date(epoch: i64) -> String {
    use chrono::TimeZone;
    match chrono::Utc.timestamp_opt(epoch, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d").to_string(),
        _ => String::from("unknown"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formats_to_iso_date() {
        // Self-verifying: derive the epoch from chrono so the constant cannot rot.
        use chrono::TimeZone;
        let dt = chrono::Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap();
        assert_eq!(epoch_to_date(dt.timestamp()), "2026-09-15");
    }
}
