//! EDGAR full-text search: primary-market filing volume.
//!
//! Endpoint: `https://efts.sec.gov/LATEST/search-index?q=&forms=<FORM>&dateRange=custom&startdt=<A>&enddt=<B>`
//! → `{"hits":{"total":{"value":N,...}}}`. Verified 2026-09-17 from this host; it
//! answers 200 with parseable JSON and needs a descriptive User-Agent like the
//! rest of EDGAR.
//!
//! WHY THIS EXISTS. The config used to declare that a broad IPO wave was
//! "unmeasurable free" and that a five-name mega-cap cohort "cannot see" it. That
//! was too pessimistic: registrations are countable. This supplies the
//! primary-market supply signal that no per-company concept can.
//!
//! WHAT IT MEASURES, AND WHAT IT DOES NOT.
//!
//! It counts FILINGS, not dollars and not outcomes. A registration statement is a
//! company's *intent* to sell stock, and many are withdrawn, postponed or priced
//! far below the range. It cannot see offer price, first-day pop, greenshoe
//! exercise, or how many of these companies had no revenue. It is a count of
//! attempted supply, and it is blind to whether the market absorbed any of it.
//!
//! It is also UNADJUSTED for market conditions. A raw count rises when many
//! companies are simply eligible to file, so this is a LEVEL-and-direction
//! measure with a long-run baseline for comparison, not a signal that a high
//! number today means the same thing it meant in a different rate environment.
//!
//! The one comparison it supports well is historical: measured from this host,
//! annual S-1 counts run 1,867 (2019), 2,890 (2020), **5,619 (2021)**, 2,715
//! (2022), 2,553 (2023), 2,663 (2024), 2,824 (2025). The 2021 spike is the
//! clearest primary-market mania in the available record, which makes this a
//! genuinely informative series rather than a decorative one.

use crate::http::Fetcher;
use crate::model::{Point, Provenance, Series};

/// A descriptive UA is required by EDGAR policy, and a generic one gets throttled.
const UA: &str = "bubble-watch/0.1 (research tool; contact via repository)";

/// Query one form type over one date range and return the hit count.
///
/// A non-200 or an unparseable body is an error, never a zero: "the count is
/// zero" and "the count could not be obtained" are different facts, and treating
/// the second as the first would silently report a collapsed primary market.
pub fn count(f: &Fetcher, form: &str, start: &str, end: &str) -> Result<u64, String> {
    let url = format!(
        "https://efts.sec.gov/LATEST/search-index?q=&forms={}&dateRange=custom&startdt={}&enddt={}",
        form, start, end
    );
    let body = f.get(&url, UA)?;
    parse_total(&body).ok_or_else(|| {
        format!(
            "full-text search returned a body without hits.total.value for form {} ({} to {}); \
             treated as a failure rather than as zero filings",
            form, start, end
        )
    })
}

/// Pull `hits.total.value` out of the response, tolerating the two shapes EDGAR
/// uses for `total` (a bare number in older responses, an object with `value`
/// in current ones).
pub fn parse_total(body: &str) -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let total = v.get("hits")?.get("total")?;
    if let Some(n) = total.as_u64() {
        return Some(n);
    }
    total.get("value")?.as_u64()
}

/// Calendar dates bounding a trailing window of `days`, ending at `today`.
///
/// Kept pure and explicit so callers can test it and so the window is never a
/// hidden constant.
pub fn window(today: &str, days: i64) -> Option<(String, String)> {
    let end = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok()?;
    let start = end - chrono::Duration::days(days);
    Some((
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    ))
}

/// Fetch the S-1 registration count for a trailing window as a one-point Series,
/// so it lives in `Observations` alongside every other retrieved datum and
/// carries the same provenance contract.
///
/// This is deliberately a SINGLE point: the historical comparison is a documented
/// constant in the config rather than a scraped series, because full-text search
/// is only reliable from 2001 and issuing ~26 historical queries per run would be
/// both slow and rude to a public endpoint. The config records the reference
/// values and where they came from, so the comparison is auditable.
pub fn registrations(f: &Fetcher, today: &str, days: i64) -> Result<Series, String> {
    let (start, end) = window(today, days)
        .ok_or_else(|| format!("could not derive a {} day window ending {}", days, today))?;
    let n = count(f, "S-1", &start, &end)?;
    Ok(Series {
        provenance: Provenance {
            source: "sec-edgar-fulltext".into(),
            endpoint: format!(
                "efts.sec.gov/LATEST/search-index forms=S-1 window={}..={}",
                start, end
            ),
            as_of: end,
            retrieved_at: crate::now_iso8601(),
        },
        points: vec![Point {
            date: start,
            value: n as f64,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_object_shaped_total() {
        let body = r#"{"hits":{"total":{"value":1912,"relation":"eq"},"hits":[]}}"#;
        assert_eq!(parse_total(body), Some(1912));
    }

    #[test]
    fn parses_the_bare_number_shaped_total() {
        let body = r#"{"hits":{"total":42,"hits":[]}}"#;
        assert_eq!(parse_total(body), Some(42));
    }

    #[test]
    fn a_body_without_a_total_is_none_not_zero() {
        // The critical distinction: "unknown" must never be read as "zero
        // filings", which would look like a collapsed primary market.
        assert_eq!(parse_total(r#"{"hits":{}}"#), None);
        assert_eq!(parse_total("not json at all"), None);
        assert_eq!(parse_total(r#"{"hits":{"total":{"relation":"eq"}}}"#), None);
        assert_eq!(parse_total(""), None);
    }

    #[test]
    fn window_is_the_requested_length_and_ends_on_today() {
        let (s, e) = window("2026-09-17", 365).unwrap();
        assert_eq!(e, "2026-09-17");
        assert_eq!(s, "2025-09-17");
        let days = crate::sources::edgar::days_between(&s, &e);
        assert_eq!(days, 365);
    }

    #[test]
    fn window_rejects_an_unparseable_date() {
        assert!(window("not-a-date", 365).is_none());
    }
}
