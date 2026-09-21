//! Federal Register API — US export-control activity.
//!
//! ## Why this is a FALSIFIER and not a stress indicator
//!
//! The obvious reading of "export controls" is that they constrain AI revenue, so
//! a flurry of rule-making should raise stress. The measured reality is the
//! opposite: over the twelve months to 2026-09-20 the BIS published **two** rules
//! matching "advanced computing", and the most recent one (2026-07-14) *eases*
//! access for the United Arab Emirates.
//!
//! Two documents in a year is a **quiet** policy period. Adding it to the composite
//! would be counting a non-event as stress, so it lives in the falsifier panel,
//! where a quiet period reads as **counter-evidence** to the constraint narrative.
//!
//! ## Verified behaviour (probed live 2026-09-20, keyless)
//!
//! | query | status | count |
//! |---|---|---|
//! | `term="advanced computing"` + BIS agency, all time | 200 | **27** |
//! | same + `publication_date >=` one year ago | 200 | **2** |
//!
//! The date filter is what makes the number meaningful, and it is applied here.
//! The unfiltered 27 spans years and would misrepresent the recent pace.

use crate::http::{Fetcher, UA_WEB};

/// The base document-search endpoint. Recorded in provenance verbatim.
pub const QUERY_URL: &str = "https://www.federalregister.gov/api/v1/documents.json";

/// Build the request for BIS rules matching a term since a date.
///
/// The API needs bracketed condition keys, which are percent-encoded here rather
/// than left raw so the URL is safe in a shell, a log, and an HTTP client.
pub fn query_for(term: &str, since: &str) -> String {
    let t = term.replace(' ', "+");
    format!(
        "{QUERY_URL}?per_page=1&order=newest\
         &conditions%5Bterm%5D=%22{t}%22\
         &conditions%5Bagencies%5D%5B%5D=industry-and-security-bureau\
         &conditions%5Bpublication_date%5D%5Bgte%5D={since}"
    )
}

/// Parse `count` and the newest publication date from a search response.
///
/// Returns an error when `count` is absent. A missing count is NOT zero: zero
/// would read as "no export-control activity", which is a strong claim, whereas a
/// missing field means the response shape changed.
pub fn parse_count(body: &str) -> Result<(u32, Option<String>), String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("federal register unparseable: {e}"))?;
    let count = v.get("count").and_then(|c| c.as_u64()).ok_or_else(|| {
        "no `count` field in the Federal Register response — refusing to report a \
             zero rule count from a response that did not supply one"
            .to_string()
    })?;
    let latest = v
        .get("results")
        .and_then(|r| r.as_array())
        .and_then(|a| a.first())
        .and_then(|d| d.get("publication_date"))
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());
    Ok((count as u32, latest))
}

/// Count BIS rules matching "advanced computing" published in the last year.
///
/// `today` is passed in (ISO `YYYY-MM-DD`) so this is deterministic and testable
/// rather than depending on the wall clock.
pub fn bis_rule_count_since(f: &Fetcher, today: &str) -> Result<(u32, Option<String>), String> {
    let since = one_year_before(today);
    let body = f.get(&query_for("advanced computing", &since), UA_WEB)?;
    parse_count(&body)
}

/// Convenience wrapper using the run clock.
pub fn bis_rule_count(f: &Fetcher) -> Result<(u32, Option<String>), String> {
    let today = crate::history::date_of(&crate::now_iso8601());
    bis_rule_count_since(f, &today)
}

/// `YYYY-MM-DD` one year before the given date.
///
/// Pure string arithmetic on the year field: the query only needs a lower bound,
/// and a leap-day-exact calendar subtraction would add a date dependency for no
/// gain. Returning the same day-of-year one year earlier is close enough for a
/// count whose nearest alternative match is months away.
pub fn one_year_before(date: &str) -> String {
    match date.split('-').next().and_then(|y| y.parse::<i32>().ok()) {
        Some(y) => format!("{}{}", y - 1, &date[4.min(date.len())..]),
        None => date.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_count_is_an_error_not_a_zero() {
        // Zero would read as "no export-control activity at all", a strong claim.
        // A missing field means the response changed shape.
        let e = parse_count(r#"{"results":[]}"#).unwrap_err();
        assert!(e.contains("count"), "must name the missing field: {}", e);
        assert!(
            e.contains("refusing"),
            "must say it is refusing rather than reporting zero: {}",
            e
        );
    }

    #[test]
    fn count_and_newest_date_are_extracted() {
        let body = r#"{"count":2,"results":[
          {"publication_date":"2026-07-14","type":"Rule","title":"Enhanced Favorable Treatment"}
        ]}"#;
        let (n, latest) = parse_count(body).unwrap();
        assert_eq!(n, 2);
        assert_eq!(latest.as_deref(), Some("2026-07-14"));
    }

    #[test]
    fn a_zero_count_is_reported_as_zero_when_the_field_says_so() {
        // Distinguished from a MISSING count: an explicit 0 is a measurement.
        let (n, latest) = parse_count(r#"{"count":0,"results":[]}"#).unwrap();
        assert_eq!(n, 0);
        assert!(latest.is_none());
    }

    #[test]
    fn the_one_year_window_is_computed_not_hardcoded() {
        // A hardcoded lower bound would silently widen as time passes and start
        // counting rules from years ago.
        assert_eq!(one_year_before("2026-09-20"), "2025-09-20");
        assert_eq!(one_year_before("2026-01-01"), "2025-01-01");
        // Malformed input is passed through rather than producing a nonsense year.
        assert_eq!(one_year_before("nonsense"), "nonsense");
    }

    #[test]
    fn the_query_encodes_bracketed_condition_keys() {
        // Raw brackets in a URL are unsafe in a shell and some clients; the API
        // requires the encoded form.
        let q = query_for("advanced computing", "2025-09-20");
        assert!(
            q.contains("conditions%5Bterm%5D="),
            "term key must be encoded"
        );
        assert!(q.contains("conditions%5Bpublication_date%5D%5Bgte%5D=2025-09-20"));
        assert!(!q.contains('['), "no raw brackets: {}", q);
        assert!(q.contains("advanced+computing"), "spaces encoded: {}", q);
    }

    #[test]
    fn an_unparseable_body_says_so_rather_than_panicking() {
        for bad in ["", "not json", "[]"] {
            assert!(parse_count(bad).is_err());
        }
    }
}
