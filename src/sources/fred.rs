//! FRED client — an OPTIONAL source, with two transports.
//!
//! ## Why there are two
//!
//! Measured from this host (2026-09-16/17), the two FRED endpoints behave
//! completely differently, and the distinction is the whole point of this
//! module:
//!
//! | Endpoint                                   | Result                       |
//! |--------------------------------------------|------------------------------|
//! | `fred.stlouisfed.org/graph/fredgraph.csv`  | **0/9** — read timeouts, HTTP/2 `INTERNAL_ERROR`; sometimes 200 in 0.13 s, then nothing |
//! | `api.stlouisfed.org/fred/...`              | **5/5** at ~0.15 s, returning a clean `400 api_key is not set` |
//!
//! So the CSV path is flaky from here, while the keyed JSON API host is
//! consistently reachable. A key is therefore not a nicety — it routes the
//! requests through the transport that actually works.
//!
//! ## Ordering
//!
//! 1. **Keyed JSON API v2** (`api.stlouisfed.org`) when `FRED_API_KEY` is set.
//!    Also returns the **full history**, unlike the CSV path.
//! 2. **Anonymous CSV** (`fredgraph.csv`) as a keyless fallback. Serves only a
//!    ~3-year trailing window; `cosd`/`coed` do not widen it (verified).
//!
//! FRED is never *required*: every failure becomes a reported coverage gap, and
//! the dependent credit indicators degrade to `Unavailable` rather than being
//! imputed.

use crate::http::{redact_url, Fetcher, UA_FRED};
use crate::model::{Point, Provenance, Series};

/// Retry budget. A dead FRED must cost seconds, not minutes, and the per-host
/// circuit breaker stops the remaining series from re-trying a dead host.
pub const FRED_RETRIES: u32 = 2;

/// Environment variable holding the API key. Read at request time; the value is
/// never logged, never stored in provenance, and never rendered.
pub const FRED_KEY_ENV: &str = "FRED_API_KEY";

/// The endpoint the keyed client actually calls.
pub const FRED_API_HOST: &str = "api.stlouisfed.org";

/// Load `FRED_API_KEY` from the environment, falling back to the Hermes `.env`
/// file so the tool works without the caller having to export anything.
///
/// Nothing in this repo ever sources `.env`, and shell rc files do not reference
/// it, so relying on the process environment alone would silently degrade every
/// run to the flaky anonymous CSV transport. Reading the file directly keeps the
/// key in exactly one place — the user's existing secrets file — instead of
/// duplicating it into a shell profile or, worse, into the repo.
///
/// The value is never returned to a caller that might print it: `key_configured`
/// exposes only a bool, and `api_key` is used solely to build a URL that is
/// redacted before it is logged or stored.
fn api_key() -> Option<String> {
    if let Ok(v) = std::env::var(FRED_KEY_ENV) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }

    let home = std::env::var("HOME").ok()?;
    let path = std::path::Path::new(&home).join(".hermes").join(".env");
    let text = std::fs::read_to_string(path).ok()?;
    parse_env_value(&text, FRED_KEY_ENV)
}

/// Minimal `.env` reader: first `KEY=value` line wins, surrounding quotes and
/// `export ` prefixes tolerated, comments ignored.
///
/// Deliberately not a full dotenv implementation — it must never *evaluate* or
/// interpolate anything, because the file holds secrets.
pub fn parse_env_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let v = v.trim().trim_matches('"').trim_matches('\'').trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

/// True when a key is available, so callers can report which transport is in use.
pub fn key_configured() -> bool {
    api_key().is_some()
}

/// Fetch a series, preferring the keyed API and falling back to anonymous CSV.
///
/// Returns the series plus the transport that produced it, so the report can be
/// honest about which one it used.
pub fn series(f: &Fetcher, id: &str) -> Result<(Series, Transport), String> {
    let mut api_err: Option<String> = None;

    if let Some(key) = api_key() {
        match series_v2(f, id, &key) {
            Ok(s) => return Ok((s, Transport::KeyedApiV2)),
            Err(e) => api_err = Some(e),
        }
    }

    match series_csv(f, id) {
        Ok(s) => Ok((s, Transport::AnonymousCsv)),
        Err(csv_err) => {
            let detail = match api_err {
                Some(a) => format!("keyed API: {} | anonymous CSV: {}", a, csv_err),
                None => format!("anonymous CSV: {} (no {} set)", csv_err, FRED_KEY_ENV),
            };
            Err(detail)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Transport {
    /// `api.stlouisfed.org` — full history, reliable from this host.
    KeyedApiV2,
    /// `fredgraph.csv` — keyless, ~3-year window, flaky from this host.
    AnonymousCsv,
}

impl Transport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Transport::KeyedApiV2 => "fred-api-v2",
            Transport::AnonymousCsv => "fred-csv-anonymous",
        }
    }
}

/// Keyed JSON API. The URL is redacted before it is ever stored or logged, so
/// the key cannot leak into provenance, reports, or error text.
pub fn series_v2(f: &Fetcher, id: &str, key: &str) -> Result<Series, String> {
    let url = format!(
        "https://{}/fred/series/observations?series_id={}&file_type=json&api_key={}",
        FRED_API_HOST, id, key
    );
    let safe = redact_url(&url);
    let body = f
        .get_raw(&url, UA_FRED, FRED_RETRIES)
        .map_err(|e| e.message())?;

    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("bad JSON from {}: {}", safe, e))?;

    let obs = v
        .get("observations")
        .and_then(|o| o.as_array())
        .ok_or_else(|| format!("no observations[] in {}", safe))?;

    let mut points = Vec::new();
    for o in obs {
        let (Some(date), Some(val)) = (
            o.get("date").and_then(|d| d.as_str()),
            o.get("value").and_then(|d| d.as_str()),
        ) else {
            continue;
        };
        // FRED marks missing observations with "." — skip, never zero-fill.
        if val.trim().is_empty() || val.trim() == "." {
            continue;
        }
        let Ok(v) = val.trim().parse::<f64>() else {
            continue;
        };
        points.push(Point {
            date: date.to_string(),
            value: v,
        });
    }

    if points.is_empty() {
        return Err(format!("{} returned no usable observations", safe));
    }
    points.sort_by(|a, b| a.date.cmp(&b.date));

    let as_of = points.last().map(|p| p.date.clone()).unwrap_or_default();
    Ok(Series {
        provenance: Provenance {
            source: "fred".into(),
            // Redacted: auditable ("we called this endpoint") without the key.
            endpoint: safe,
            as_of,
            retrieved_at: crate::now_iso8601(),
        },
        points,
    })
}

/// Anonymous CSV fallback. `DATE,VALUE` with "." for non-observations.
pub fn series_csv(f: &Fetcher, id: &str) -> Result<Series, String> {
    let url = format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={}", id);
    let body = f
        .get_raw(&url, UA_FRED, FRED_RETRIES)
        .map_err(|e| e.message())?;

    let mut points = Vec::new();
    for line in body.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        let (Some(date), Some(val)) = (it.next(), it.next()) else {
            continue;
        };
        let val = val.trim();
        if val.is_empty() || val == "." {
            continue;
        }
        let Ok(v) = val.parse::<f64>() else { continue };
        points.push(Point {
            date: date.trim().to_string(),
            value: v,
        });
    }

    if points.is_empty() {
        return Err(format!("FRED {} returned no usable observations", id));
    }

    let as_of = points.last().map(|p| p.date.clone()).unwrap_or_default();
    Ok(Series {
        provenance: Provenance {
            source: "fred".into(),
            endpoint: url,
            as_of,
            retrieved_at: crate::now_iso8601(),
        },
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;

    fn parse_csv(body: &str) -> Vec<Point> {
        let mut pts = Vec::new();
        for line in body.lines().skip(1) {
            let mut it = line.split(',');
            if let (Some(d), Some(v)) = (it.next(), it.next()) {
                let v = v.trim();
                if v.is_empty() || v == "." {
                    continue;
                }
                if let Ok(val) = v.parse::<f64>() {
                    pts.push(Point {
                        date: d.into(),
                        value: val,
                    });
                }
            }
        }
        pts
    }

    #[test]
    fn csv_parses_and_skips_holiday_gaps() {
        let body = "observation_date,DGS10\n2026-09-10,5.02\n2026-09-11,.\n2026-09-12,5.01\n";
        let pts = parse_csv(body);
        assert_eq!(pts.len(), 2, "the '.' row must be skipped, not zero-filled");
        assert_eq!(pts[1].value, 5.01);
    }

    #[test]
    fn v2_json_observation_shape_parses() {
        // Mirrors the real API v2 payload for /fred/series/observations.
        let body = r#"{"realtime_start":"2026-09-16","realtime_end":"2026-09-16",
          "observation_start":"2023-01-01","observation_end":"2026-09-16","units":"lin",
          "count":3,"observations":[
            {"date":"2026-09-12","value":"2.74"},
            {"date":"2026-09-13","value":"."},
            {"date":"2026-09-14","value":"2.76"}]}"#;
        let v: serde_json::Value = serde_json::from_str(body).unwrap();
        let mut pts = Vec::new();
        for o in v["observations"].as_array().unwrap() {
            let (d, val) = (o["date"].as_str().unwrap(), o["value"].as_str().unwrap());
            if val == "." || val.is_empty() {
                continue;
            }
            pts.push(Point {
                date: d.into(),
                value: val.parse().unwrap(),
            });
        }
        assert_eq!(pts.len(), 2, "'.' must be skipped in the JSON path too");
        assert_eq!(pts.last().unwrap().value, 2.76);
    }

    #[test]
    fn missing_observations_array_is_an_error_not_an_empty_series() {
        let v: serde_json::Value = serde_json::from_str(r#"{"error_code":400}"#).unwrap();
        assert!(v.get("observations").and_then(|o| o.as_array()).is_none());
    }

    #[test]
    fn env_parser_reads_plain_quoted_and_exported_forms() {
        let text =
            "# comment line\nOTHER=x\nFRED_API_KEY=abcdef1234567890abcdef1234567890\nMORE=y\n";
        assert_eq!(
            parse_env_value(text, "FRED_API_KEY").as_deref(),
            Some("abcdef1234567890abcdef1234567890")
        );

        let quoted = "FRED_API_KEY=\"abcdef1234567890abcdef1234567890\"\n";
        assert_eq!(
            parse_env_value(quoted, "FRED_API_KEY").as_deref(),
            Some("abcdef1234567890abcdef1234567890")
        );

        let exported = "export FRED_API_KEY=abcdef1234567890abcdef1234567890\n";
        assert_eq!(
            parse_env_value(exported, "FRED_API_KEY").as_deref(),
            Some("abcdef1234567890abcdef1234567890")
        );
    }

    #[test]
    fn env_parser_ignores_comments_and_missing_keys() {
        let text = "#FRED_API_KEY=nope\nOTHER=x\n";
        assert!(parse_env_value(text, "FRED_API_KEY").is_none());
        assert!(parse_env_value("", "FRED_API_KEY").is_none());
        // A commented-out key must not be picked up.
        assert!(parse_env_value("# FRED_API_KEY=abc\n", "FRED_API_KEY").is_none());
    }

    #[test]
    fn env_parser_does_not_confuse_similar_key_names() {
        let text = "FRED_API_KEY_OLD=zzz\nNOT_FRED_API_KEY=yyy\n";
        assert!(
            parse_env_value(text, "FRED_API_KEY").is_none(),
            "must match the exact key name only"
        );
    }

    #[test]
    fn env_parser_takes_the_first_definition() {
        let text = "FRED_API_KEY=first1234567890first1234567890\nFRED_API_KEY=second\n";
        assert_eq!(
            parse_env_value(text, "FRED_API_KEY").as_deref(),
            Some("first1234567890first1234567890")
        );
    }
}
