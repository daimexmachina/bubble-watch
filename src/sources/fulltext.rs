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
const UA: &str = "bubble-watch/0.1 (research; contact: research@example.invalid)";

/// Query one form type over one date range and return the hit count.
///
/// A non-200 or an unparseable body is an error, never a zero: "the count is
/// zero" and "the count could not be obtained" are different facts, and treating
/// the second as the first would silently report a collapsed primary market.
pub fn count(f: &Fetcher, form: &str, start: &str, end: &str) -> Result<u64, String> {
    count_query(f, "", form, start, end)
}

/// As `count`, but with an explicit `q=` value.
///
/// WHY THIS IS SEPARATE: `count` originally hardcoded `q=` as empty, so passing a
/// phrase to it put the phrase into the `forms=` parameter instead. EDGAR then tried
/// to parse the phrase as a JSON array of form types and rejected the whole request
/// with HTTP 400. The bug was in the parameter wiring, not in the encoding.
pub fn count_query(
    f: &Fetcher,
    query: &str,
    form: &str,
    start: &str,
    end: &str,
) -> Result<u64, String> {
    let url = format!(
        "https://efts.sec.gov/LATEST/search-index?q={}&forms={}&dateRange=custom&startdt={}&enddt={}",
        query, form, start, end
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

/// Count filings of one form type mentioning a phrase over a calendar year.
///
/// This is a CENSUS, not a sample: every filing of that form with the SEC in the
/// period is indexed, so there is no sampling error and no panel-selection bias.
/// That is what makes it usable as a hype measure when most alternatives (news
/// volume, social chatter, search interest) are samples of unclear provenance.
///
/// The phrase must be quoted so the search treats it as a phrase rather than as
/// separate terms.
pub fn phrase_census(f: &Fetcher, phrase: &str, form: &str, year: i32) -> Result<u64, String> {
    // The phrase must be wrapped in quotes so EDGAR treats it as a phrase, and the
    // whole quoted expression percent-encoded. Building `%22phrase%22` and passing it
    // raw made the server reject the query with HTTP 400 ("Could not parse payload
    // into json"), because the unencoded quotes broke its parameter parsing.
    let encoded = percent_encode_phrase(phrase);
    count_query(
        f,
        &encoded,
        form,
        &format!("{}-01-01", year),
        &format!("{}-12-31", year),
    )
    .map_err(|e| format!("{} {} {}: {}", phrase, form, year, e))
}

/// Percent-encode a phrase for the `q=` parameter, including the surrounding quotes
/// that make it a phrase rather than a set of separate terms.
///
/// Verified against the live endpoint: passing `%22phrase%22` where the quotes are
/// themselves left raw produces HTTP 400 ("Could not parse payload into json").
pub fn percent_encode_phrase(phrase: &str) -> String {
    let mut out = String::from("%22");
    for b in phrase.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out.push_str("%22");
    out
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
    fn phrase_encoding_wraps_in_quotes_and_encodes_spaces() {
        assert_eq!(
            percent_encode_phrase("artificial intelligence"),
            "%22artificial+intelligence%22"
        );
        assert_eq!(percent_encode_phrase("bubble"), "%22bubble%22");
        // A reserved character must not leak through raw.
        assert!(percent_encode_phrase("a&b").contains("%26"));
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
