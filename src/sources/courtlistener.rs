//! CourtListener — filed dockets as HARD opposition data.
//!
//! ## Why a docket is not sentiment
//!
//! A filed lawsuit or an adopted moratorium is a **recorded event with a docket
//! number**. It cannot be astroturfed, it is not an opinion, and it is a direct
//! mechanism by which opposition reduces future revenue: a project that is
//! enjoined does not get built, and a moratorium removes the permit path entirely.
//! That is why this leg carries the scored weight while the news-tone leg does not.
//!
//! ## Verified behaviour (probed live 2026-09-26, keyless)
//!
//! | query | window | status | count |
//! |---|---|---|---|
//! | `"data center" moratorium` | all time | 200 | **331** |
//! | `"data center" moratorium` | 2025-01-01 → 2026-09-26 | 200 | **33** |
//! | `"data center" lawsuit` | 2026-01-01 → 2026-09-26 | 200 | **169** |
//!
//! No API key, and no rate-limit headers are exposed. Still throttled defensively.
//!
//! ## The scoring decision this module exists to make possible
//!
//! A count is **not** a series, and the raw level is not scoreable: the court corpus
//! grows every year, so a rising count is expected and carries no information. What
//! is comparable is **two equal-length windows**, so this module measures the
//! **year-over-year change** between a trailing 365-day window and the 365 days
//! before it. That mirrors how `narrative_saturation` handles the same problem in
//! the SEC filing census.
//!
//! ## Limits, stated because they bound the reading
//!
//! This is a **US court corpus**. Local zoning boards, county commissions and city
//! councils — where the majority of real-world data-centre resistance is actually
//! decided — do not file there. Measured dead ends for that local layer, all
//! probed: Legistar municipal legislation (HTTP 500, needs a per-client commercial
//! licence), the Socrata open-data catalog (`data center moratorium` → 0 results),
//! and FERC eLibrary (HTTP 405/404, no public endpoint). So this figure is a
//! **lower bound** on opposition pressure and must always be described as one.

use crate::http::{Fetcher, UA_WEB};
use serde::{Deserialize, Serialize};

/// The docket search endpoint. Recorded in provenance verbatim.
pub const SEARCH_URL: &str = "https://www.courtlistener.com/api/rest/v4/search/";

/// The opposition queries, as (label, query) pairs.
///
/// Data rather than inline string literals so no query can enter the model whose
/// meaning was not reviewed, and so the set is countable by a test.
pub fn tracked_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("moratorium", "\"data center\" moratorium"),
        ("lawsuit", "\"data center\" lawsuit"),
        ("injunction", "\"data center\" injunction"),
    ]
}

/// Build a date-bounded docket-search URL.
///
/// Spaces become `+` and embedded quotes are percent-encoded, so the phrase is
/// searched as a phrase rather than as two loose words.
pub fn search_url(query: &str, filed_after: &str, filed_before: &str) -> String {
    let q = query.replace(' ', "+").replace('"', "%22");
    format!("{SEARCH_URL}?q={q}&filed_after={filed_after}&filed_before={filed_before}&type=r")
}

/// Parse the `count` field of a docket-search response.
///
/// Returns an error when absent, because a missing count is **not** zero: an
/// explicit zero would be a measurement of no filings, whereas a missing field
/// means the response shape changed or the request was refused. Reporting the
/// latter as zero would read as "opposition has stopped".
pub fn parse_count(body: &str) -> Result<u64, String> {
    let trimmed = body.trim_start();
    if trimmed.starts_with('<') {
        return Err(format!(
            "CourtListener returned a non-JSON body (an error or block page), so the \
             docket count is unknown; refusing to report it as zero. Body began: {:?}",
            body.chars().take(80).collect::<String>()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("CourtListener unparseable: {e}"))?;
    v.get("count").and_then(|c| c.as_u64()).ok_or_else(|| {
        "no `count` field in the CourtListener response — refusing to report a zero \
         docket count from a response that did not supply one"
            .to_string()
    })
}

/// Count dockets for one query within a window.
pub fn count_window(
    f: &Fetcher,
    query: &str,
    filed_after: &str,
    filed_before: &str,
) -> Result<u64, String> {
    let body = f.get(&search_url(query, filed_after, filed_before), UA_WEB)?;
    parse_count(&body)
}

/// The year-over-year change in filings for one tracked query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocketChange {
    /// Which query this is, e.g. `moratorium`.
    pub query: String,
    /// Percentage change in docket count, trailing 365 days vs the 365 before.
    pub yoy_pct: f64,
    /// Filings in the trailing window.
    pub recent_count: u64,
    /// Filings in the prior window.
    pub prior_count: u64,
}

/// `YYYY-MM-DD` exactly `days` before the given date.
///
/// Uses `chrono` rather than the string arithmetic `federalregister.rs` uses,
/// because here the two windows must be the **same length** for the comparison to
/// mean anything: a one-day error on a leap year would compare a 365-day window
/// against a 366-day one and bias the ratio. That precision is worth the
/// dependency, which this crate already carries.
pub fn days_before(date: &str, days: i64) -> Option<String> {
    let d = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    Some(
        (d - chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string(),
    )
}

/// Year-over-year percentage change in docket count for one query.
///
/// Compares the trailing `window_days` against the `window_days` immediately before
/// it. Returns `None` when the prior window had **zero** filings, because a
/// percentage change from zero is undefined — and inventing a large number there
/// would report an explosion of opposition from a base that never existed. The
/// caller turns `None` into an honest gap.
pub fn year_over_year_pct(
    f: &Fetcher,
    query: &str,
    today: &str,
    window_days: i64,
) -> Result<Option<(f64, u64, u64)>, String> {
    let start_recent = days_before(today, window_days)
        .ok_or_else(|| format!("could not subtract {window_days} days from {today}"))?;
    let start_prior = days_before(today, window_days * 2)
        .ok_or_else(|| format!("could not subtract {} days from {today}", window_days * 2))?;

    let recent = count_window(f, query, &start_recent, today)?;
    let prior = count_window(f, query, &start_prior, &start_recent)?;

    if prior == 0 {
        return Ok(None);
    }
    let pct = (recent as f64 - prior as f64) / prior as f64 * 100.0;
    Ok(Some((pct, recent, prior)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_docket_search_yields_its_count() {
        assert_eq!(parse_count(r#"{"count":331,"results":[]}"#).unwrap(), 331);
        assert_eq!(parse_count(r#"{"count":0,"results":[]}"#).unwrap(), 0);
    }

    #[test]
    fn a_body_without_a_count_is_an_error_not_a_zero() {
        // The distinction that matters: an explicit 0 is a measurement, a missing
        // field is a broken response. Reporting the latter as 0 would read as
        // "opposition has stopped".
        let e = parse_count("{}").unwrap_err();
        assert!(e.contains("count"), "must name the field: {e}");
        assert!(e.contains("refusing"), "must refuse rather than guess: {e}");
    }

    #[test]
    fn an_html_error_page_is_an_error_not_a_zero() {
        for bad in ["<html>403 Forbidden</html>", "<!DOCTYPE html><html>"] {
            let e = parse_count(bad).unwrap_err();
            assert!(e.contains("non-JSON"), "{e}");
            assert!(e.contains("zero"), "must name the wrong conclusion: {e}");
        }
    }

    #[test]
    fn an_unparseable_body_says_so_rather_than_panicking() {
        for bad in ["", "not json", "[]"] {
            assert!(parse_count(bad).is_err(), "{bad:?} must be an error");
        }
    }

    #[test]
    fn the_phrases_are_encoded_as_phrases_not_loose_words() {
        let u = search_url("\"data center\" moratorium", "2025-01-01", "2026-09-26");
        assert!(u.contains("q=%22data+center%22+moratorium"), "{u}");
        assert!(!u.contains('"'), "no raw quotes: {u}");
        assert!(u.contains("filed_after=2025-01-01"));
        assert!(u.contains("filed_before=2026-09-26"));
        assert!(u.contains("type=r"), "must search RECAP dockets: {u}");
    }

    #[test]
    fn the_window_arithmetic_is_exactly_n_days() {
        // Equal-length windows are the precondition for the comparison to mean
        // anything, so this is checked across a leap boundary rather than trusting
        // it.
        assert_eq!(days_before("2026-09-26", 365).unwrap(), "2025-09-26");
        assert_eq!(days_before("2026-09-26", 730).unwrap(), "2024-09-26");
        // 2024 is a leap year, and Feb 2025 has 28 days, so 366 days back from
        // 2025-03-01 lands on 2024-02-29 rather than 2024-03-01. (The first version
        // of this assertion said 2024-03-01 and was WRONG — the code was right.)
        assert_eq!(days_before("2025-03-01", 366).unwrap(), "2024-02-29");
        // And the leap day is what makes the 365-day answer differ.
        assert_eq!(days_before("2025-03-01", 365).unwrap(), "2024-03-01");
    }

    #[test]
    fn an_unparseable_date_yields_none_rather_than_a_nonsense_window() {
        assert!(days_before("nonsense", 365).is_none());
        assert!(days_before("", 365).is_none());
    }

    #[test]
    fn the_tracked_queries_are_non_empty_and_distinct() {
        let qs = tracked_queries();
        assert!(!qs.is_empty());
        let labels: std::collections::HashSet<_> = qs.iter().map(|(l, _)| *l).collect();
        assert_eq!(labels.len(), qs.len(), "labels must be distinct");
        for (label, q) in qs {
            assert!(!q.trim().is_empty(), "{label} is empty");
            assert!(
                q.contains("data center"),
                "{label} must anchor on the data-centre phrase: {q}"
            );
        }
    }
}
