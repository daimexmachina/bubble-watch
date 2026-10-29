//! Polymarket — party-control probabilities, used as a POLITICAL-RISK input.
//!
//! ## Why this exists
//!
//! The model had no political or regulatory-probability channel at all. The closest thing was the BIS
//! export-control count in `federalregister.rs`, which lives in the FALSIFIER panel precisely because
//! the measured rule-making pace was quiet (two rules in a year).
//!
//! Daim's thesis is a chain: a Democratic sweep leads to investigations, investigations lead to the AI
//! boom being reined in, and that is a crash risk. Only the FIRST link is directly observable, and this
//! module measures it. The other links are assumptions and are labelled as such wherever this feeds a
//! score — the tool's own contract says it reports STATE, not forecasts, and an election probability is
//! the one input here that is explicitly forward-looking.
//!
//! ## The API
//!
//! Gamma is keyless, read-only and generous (4000 requests / 10s), so two calls per run is nothing.
//! Prices ARE probabilities: a Yes price of 0.915 means the market thinks 91.5% likely. The fields
//! `outcomePrices` and `outcomes` come back as JSON-encoded STRINGS inside the JSON, so they are
//! parsed twice.
//!
//! Measured 2026-10-27: House Democrats 91.5% on $24.1M of volume, Senate Democrats 65.5% on $9.6M.

use crate::http::{Fetcher, UA_WEB};

/// Base endpoint for market discovery.
pub const GAMMA_SEARCH: &str = "https://gamma-api.polymarket.com/public-search";

/// Percent-encode a query term. Deliberately minimal: these queries are letters and spaces, so
/// escaping a fixed set is enough and avoids pulling in a URL crate for two calls.
fn encode(q: &str) -> String {
    q.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '?' | '&' | '=' | '#' | '+' | '%' => format!("%{:02X}", c as u32),
            other => other.to_string(),
        })
        .collect()
}

/// The probability, in percent, that the Democratic Party controls the chamber after the midterms.
///
/// Refuses rather than guessing. A market that is missing, has no two-outcome price pair, or reports a
/// price outside 0..1 is an ERROR and not a zero — a zero would read as "no chance of a sweep", which is
/// a claim this module has not earned.
pub fn control_probability(f: &Fetcher, chamber: &str) -> Result<f64, String> {
    let q = format!("Which party will win the {chamber} in 2026");
    let url = format!("{}?q={}&limit_per_type=5", GAMMA_SEARCH, encode(&q));
    let body = f
        .get(&url, UA_WEB)
        .map_err(|e| format!("polymarket search for the {chamber} failed: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("polymarket search for the {chamber} returned non-JSON: {e}"))?;

    let events = v
        .get("events")
        .and_then(|e| e.as_array())
        .ok_or_else(|| format!("polymarket search for the {chamber} had no `events` array"))?;

    for ev in events {
        let Some(markets) = ev.get("markets").and_then(|m| m.as_array()) else {
            continue;
        };
        for m in markets {
            let question = m.get("question").and_then(|q| q.as_str()).unwrap_or("");
            // The Democratic contract is the one that names the party AND the chamber AND the word
            // control. Matching on the label rather than the array position is deliberate: position
            // would silently return the Republican price if the ordering ever changed.
            if !question.contains("Democratic") || !question.contains(chamber) {
                continue;
            }
            let Some(prices) = m.get("outcomePrices").and_then(|p| p.as_str()) else {
                continue;
            };
            let parsed: Vec<String> = serde_json::from_str(prices)
                .map_err(|e| format!("outcomePrices is not a list: {e}"))?;
            let Some(first) = parsed.first() else {
                continue;
            };
            let p: f64 = first
                .parse()
                .map_err(|e| format!("outcome price {first:?} is not a number: {e}"))?;
            if !(0.0..=1.0).contains(&p) {
                return Err(format!(
                    "polymarket returned a price of {p} for the {chamber}, which is outside 0..1 and \
                     cannot be a probability"
                ));
            }
            return Ok(p * 100.0);
        }
    }
    Err(format!(
        "no Democratic-control contract for the {chamber} was found in the search results"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_encoder_escapes_what_would_break_the_url() {
        assert_eq!(encode("Which party"), "Which%20party");
        assert_eq!(encode("a&b"), "a%26b");
    }
}
