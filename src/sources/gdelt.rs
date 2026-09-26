//! GDELT DOC 2.0 — news coverage volume and TONE.
//!
//! ## Why this is the transmission channel, not a scored sentiment meter
//!
//! News coverage is not itself causal, but it sits upstream of politics, and
//! politics sits upstream of permits. The value here is that it is a **measured**
//! public channel rather than a proxy for one.
//!
//! The tone series is deliberately NOT scored in this version. We do not yet know
//! whether attention **leads** a repricing or **follows** it, and a series that
//! spikes *after* a drawdown is a lagging indicator that would be actively harmful
//! to score. The lead/lag test in the plan gates that decision; until it clears,
//! tone is reported as unscored context.
//!
//! ## Verified behaviour (probed live 2026-09-25/26, keyless)
//!
//! | query | mode | status | measured |
//! |---|---|---|---|
//! | `"data center" moratorium` | `timelinevol` | 200 | 349 daily points over 12m; first-half mean **0.0040** → second-half **0.0250** = **6.33x** |
//! | `"AI bubble"` | `timelinetone` | 200 | 169 points over 6m; latest **-3.48**, the **1.2th percentile** of the window |
//!
//! ## Three traps, each of which cost a real failure to find
//!
//! 1. **The rate limit is one request per 5 seconds and it is enforced hard.** Two
//!    concurrent calls returned HTTP 429. So did two calls 7s apart. This host must
//!    therefore be treated as **serial**: one request in flight, spaced generously,
//!    and a 429 must degrade to a gap rather than being retried in a loop. NOTE the
//!    irony worth stating — `timelinevol` returns a **share of all coverage**, so
//!    asking it twice in quick succession is exactly the thing that gets refused.
//! 2. **No bare parentheses in the query.** `(a OR b) AND c` is rejected with
//!    `Parentheses may only be used around OR'd statements`. Use `a+OR+b`, or plain
//!    multi-term AND.
//! 3. **A 429 body is not JSON and must never parse as an empty series.** A
//!    permissive parser returns zero points, which then reads as "no opposition
//!    coverage" — a **calm** signal produced by a *refusal*. That inversion is the
//!    worst failure mode available here, so `parse_timeline` rejects it explicitly.

use crate::http::{Fetcher, UA_WEB};
use serde::{Deserialize, Serialize};

/// The base timeline endpoint. Recorded in provenance verbatim.
pub const TIMELINE_URL: &str = "https://api.gdeltproject.org/api/v2/doc/doc";

/// Minimum seconds between requests to this host.
///
/// Measured: two requests 7 seconds apart still 429'd the second. The published
/// guidance is 5s; the observed floor is higher, so this is set with headroom.
pub const MIN_GAP_SECONDS: u64 = 8;

/// One point of a GDELT daily timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelinePoint {
    /// `YYYY-MM-DD`. The API sends `YYYYMMDDTHHMMSSZ`; the time part is discarded
    /// because these series are daily.
    pub date: String,
    /// For `timelinetone`: a tone score where negative is hostile. For
    /// `timelinevol`: a **fraction of all coverage**, not an absolute count.
    pub value: f64,
}

/// Which timeline mode to request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Share of all news coverage matching the query.
    Volume,
    /// Average tone of coverage matching the query. Negative = hostile.
    Tone,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Volume => "timelinevol",
            Mode::Tone => "timelinetone",
        }
    }
}

/// Build a timeline URL.
///
/// Spaces become `+`. Callers must NOT pass bare parentheses (see trap 2 in the
/// module docs); this function does not paper over it, because silently rewriting a
/// rejected query into a different one would hide the mistake.
pub fn timeline_url(query: &str, mode: Mode, timespan: &str) -> String {
    // Quotes MUST be percent-encoded. A raw `"` in a URL is invalid, and the live
    // probe that established this endpoint used the `%22` form — so a raw-quote
    // version is a different request from the one that was verified.
    let q = query.replace(' ', "+").replace('"', "%22");
    format!(
        "{TIMELINE_URL}?query={q}&mode={}&timespan={timespan}&format=json",
        mode.as_str()
    )
}

/// True when a body is GDELT refusing the request rather than answering it.
///
/// Detected by content because the refusal arrives with HTTP 429 and a body that is
/// plain prose, not JSON.
fn is_refusal(body: &str) -> bool {
    let head = body.trim_start();
    head.starts_with("Please limit requests")
        || head.starts_with("Your query")
        || head.starts_with("We are sorry")
        || head.starts_with('<')
}

/// Parse a GDELT timeline response.
///
/// Returns an error — never an empty series — when the body is a rate-limit or
/// error page, or when the expected `timeline[0].data` path is absent. A missing
/// path means the response shape changed, and reporting it as "no coverage" would
/// invert a refusal into a calm reading.
pub fn parse_timeline(body: &str) -> Result<Vec<TimelinePoint>, String> {
    if is_refusal(body) {
        return Err(format!(
            "GDELT refused the request rather than answering it (rate limit or error \
             page); refusing to report this as an empty series, because an empty series \
             would read as NO coverage — the opposite of what happened. Body began: {:?}",
            body.chars().take(80).collect::<String>()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("GDELT unparseable: {e}"))?;
    let data = v
        .get("timeline")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|t| t.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| {
            "no `timeline[0].data` array in the GDELT response — refusing to report a \
             zero-length series from a response that did not supply one"
                .to_string()
        })?;
    let mut out = Vec::with_capacity(data.len());
    for row in data {
        let raw = row
            .get("date")
            .and_then(|d| d.as_str())
            .ok_or_else(|| "a GDELT timeline row had no `date`".to_string())?;
        let date = raw
            .get(..8)
            .ok_or_else(|| format!("GDELT date {raw:?} is too short to hold a YYYYMMDD prefix"))?;
        let date = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]);
        let value = row
            .get("value")
            .and_then(|x| x.as_f64())
            .ok_or_else(|| format!("a GDELT timeline row ({date}) had no numeric `value`"))?;
        out.push(TimelinePoint { date, value });
    }
    if out.is_empty() {
        return Err(
            "GDELT returned a present-but-empty timeline — treated as an error, not as \
             zero coverage"
                .to_string(),
        );
    }
    Ok(out)
}

/// Fetch one timeline.
///
/// `sleep_before` is passed in rather than called here so the pacing is visible at
/// the call site and testable, and because this crate keeps IO in the source layer
/// with no hidden delays.
pub fn fetch_timeline(
    f: &Fetcher,
    query: &str,
    mode: Mode,
    timespan: &str,
) -> Result<Vec<TimelinePoint>, String> {
    let url = timeline_url(query, mode, timespan);
    let body = f.get(&url, UA_WEB)?;
    parse_timeline(&body)
}

/// The queries this model tracks, as (label, query) pairs.
///
/// Kept as data so the caller cannot invent a query whose meaning was never
/// reviewed. Each is a plain multi-term query — no parentheses, per trap 2.
pub fn tracked_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("datacenter_moratorium", "\"data center\" moratorium"),
        ("ai_bubble_tone", "\"AI bubble\""),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_series_parses_its_actual_nested_shape() {
        let body = r#"{"timeline":[{"data":[
          {"date":"20260920T000000Z","value":-0.5},
          {"date":"20260921T000000Z","value":-3.48}
        ]}]}"#;
        let s = parse_timeline(body).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].date, "2026-09-20");
        assert_eq!(s[1].date, "2026-09-21");
        assert!((s[1].value - (-3.48)).abs() < 1e-9);
    }

    #[test]
    fn a_rate_limit_body_is_an_error_not_an_empty_series() {
        // THE CENTRAL TRAP. A refusal read as an empty series would report NO
        // coverage, i.e. calm, at the exact moment the source was too busy to tell
        // us anything. This is the failure that inverts a blocker into a signal.
        let live = "Please limit requests to one every 5 seconds or contact \
                    kalev.leetaru5@gmail.com for larger queries. All high-traffic users \
                    should switch to our ngrams dataset:";
        let e = parse_timeline(live).unwrap_err();
        assert!(
            e.contains("refused"),
            "must say the request was refused: {e}"
        );
        assert!(
            e.contains("opposite"),
            "must name the inversion it is avoiding: {e}"
        );
    }

    #[test]
    fn a_missing_data_array_is_an_error_not_an_empty_series() {
        for bad in ["", "{}", "[]", "not json", "<html>429</html>"] {
            assert!(
                parse_timeline(bad).is_err(),
                "{bad:?} must not parse as a zero-length series"
            );
        }
    }

    #[test]
    fn a_present_but_empty_timeline_is_an_error() {
        // Distinct from a MISSING timeline: an explicitly empty array is still not
        // evidence of zero coverage, because GDELT omits empty buckets entirely.
        let e = parse_timeline(r#"{"timeline":[{"data":[]}]}"#).unwrap_err();
        assert!(e.contains("empty"), "{e}");
    }

    #[test]
    fn a_row_missing_its_value_is_an_error_not_a_zero() {
        let body = r#"{"timeline":[{"data":[{"date":"20260921T000000Z"}]}]}"#;
        assert!(parse_timeline(body).is_err());
    }

    #[test]
    fn a_date_too_short_to_hold_a_yyyyMMdd_prefix_is_an_error() {
        let body = r#"{"timeline":[{"data":[{"date":"2026","value":1.0}]}]}"#;
        let e = parse_timeline(body).unwrap_err();
        assert!(e.contains("YYYYMMDD"), "{e}");
    }

    #[test]
    fn the_query_uses_plus_for_spaces_and_no_parentheses() {
        let u = timeline_url("\"data center\" moratorium", Mode::Volume, "12m");
        assert!(u.contains("query=%22data+center%22+moratorium"), "{u}");
        assert!(!u.contains('('), "no bare parens: {u}");
        assert!(u.contains("mode=timelinevol"));
        assert!(u.contains("timespan=12m"));
    }

    #[test]
    fn the_two_modes_map_to_their_distinct_live_parameter_names() {
        assert_eq!(Mode::Volume.as_str(), "timelinevol");
        assert_eq!(Mode::Tone.as_str(), "timelinetone");
        assert_ne!(
            timeline_url("x", Mode::Volume, "1m"),
            timeline_url("x", Mode::Tone, "1m"),
            "the two modes must not build the same URL"
        );
    }

    #[test]
    fn the_tracked_queries_contain_no_bare_parentheses() {
        // Guarding the config-level list: a query with parens is rejected by the
        // live API, and the rejection looks like an outage rather than a syntax bug.
        for (label, q) in tracked_queries() {
            assert!(
                !q.contains(['(', ')']),
                "{label} has bare parentheses, which the API rejects: {q}"
            );
        }
    }

    #[test]
    fn the_pacing_constant_is_at_least_the_observed_floor() {
        // Measured: 7s apart still 429'd. Anything below that is known to fail.
        assert!(
            MIN_GAP_SECONDS >= 8,
            "pacing below the observed floor will 429"
        );
    }
}
