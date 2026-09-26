//! Wikimedia pageviews — the public-ATTENTION series, deliberately UNSCORED.
//!
//! ## Why this leg is fetched and not scored
//!
//! Pageviews are the noisiest channel the project has access to and the most prone to
//! being a **lagging curiosity signal**: people look up a topic after it is news, not
//! before. The v1.4 plan accordingly gives this series **weight 0** — observed,
//! reported, and never trusted with influence until a lead/lag test clears it.
//!
//! That test has now been run for the *sibling* sentiment channel and came back NULL
//! (peak correlation at lag **+7**, i.e. the series trails price; empirical null p90
//! 0.264 against an observed 0.221, p = 0.36). See
//! `.hermes/analysis/2026-09-26-sentiment-lead-lag.md`. Attention is a *more* lagging
//! channel than news tone, so the prior is that it would fail the same test. It is
//! fetched because a weight-0 series costs nothing in the composite — `score::composite`
//! documents that "indicators with weight 0 (declared gaps) never affect either number" —
//! and because a reader who sees an AI-bubble news cycle deserves to see whether the
//! public is actually looking, which is a different question from whether it is scared.
//!
//! ## Verified behaviour (probed live 2026-09-26, keyless)
//!
//! | article | window | status | points |
//! |---|---|---|---|
//! | `AI_bubble` | 2026-08-25 → 2026-09-24 | 200 | **31** (1785 → 1416 views/day) |
//!
//! No API key and no registration. The endpoint returns `items[]` oldest-first with
//! `timestamp` as `YYYYMMDD00` and an integer `views`.
//!
//! ## The trap this module refuses to fall into
//!
//! Pageviews have a strong **weekly** cycle and are also affected by bot traffic in the
//! `all-access` aggregate. Comparing a single day against its neighbour would therefore
//! measure the weekday, not attention. This module reports a **7-day trailing mean and
//! the change against the previous 7 days**, which cancels the weekday cycle by
//! construction. It does not attempt to correct for bots — that limit is stated instead
//! of being silently patched.

use crate::http::{Fetcher, UA_WEB};
use serde::{Deserialize, Serialize};

/// The per-article pageviews endpoint. Recorded in provenance verbatim.
pub const PAGEVIEWS_URL: &str = "https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article";

/// The tracked articles, as (label, article title) pairs.
///
/// Data rather than inline literals, so no article can enter the model whose meaning
/// was not reviewed, and so the set is countable by a test.
pub fn tracked_articles() -> Vec<(&'static str, &'static str)> {
    vec![
        ("ai_bubble", "AI_bubble"),
        ("artificial_intelligence", "Artificial_intelligence"),
    ]
}

/// Build a daily pageviews URL for one article over `[start, end]` (both `YYYYMMDD`).
///
/// The path is `/per-article/{project}/{access}/{agent}/{article}/daily/{start}/{end}`.
/// `user` rather than `all-agents` is deliberate: the `all-access` aggregate includes
/// spiders, which would make an attention series partly a measure of crawler traffic.
pub fn pageviews_url(article: &str, start: &str, end: &str) -> String {
    format!("{PAGEVIEWS_URL}/en.wikipedia/all-access/user/{article}/daily/{start}/{end}")
}

/// `YYYY-MM-DD` -> `YYYYMMDD`. The pageviews API wants the compact form, and the
/// project's date helpers return the dashed one, so the conversion is explicit here
/// rather than an implicit `replace` at the call site.
///
/// Errors on anything that is not exactly `YYYY-MM-DD`; silently stripping dashes from
/// a malformed date would send the API a window nobody intended.
pub fn compact_date(date: &str) -> Result<String, String> {
    let b = date.as_bytes();
    let shaped = b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                true
            } else {
                c.is_ascii_digit()
            }
        });
    if !shaped {
        return Err(format!(
            "{date:?} is not YYYY-MM-DD, so it cannot be converted to the compact form \
             the pageviews API requires"
        ));
    }
    Ok(format!("{}{}{}", &date[0..4], &date[5..7], &date[8..10]))
}

/// One day of pageviews.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayViews {
    /// `YYYY-MM-DD`, normalised from the API's `YYYYMMDD00`.
    pub date: String,
    pub views: u64,
}

/// Parse a pageviews response into an oldest-first daily series.
///
/// Returns an error — never an empty series — when the payload is not the expected
/// shape, because "the request was refused" and "nobody looked" are opposite findings
/// and must not collapse into the same reading. A 404 from this API arrives as a JSON
/// body with no `items` key, which is the case that would otherwise read as zero.
pub fn parse_series(body: &str) -> Result<Vec<DayViews>, String> {
    let trimmed = body.trim_start();
    if trimmed.starts_with('<') {
        return Err(format!(
            "Wikimedia returned a non-JSON body (an error or block page), so the \
             pageview series is unknown; refusing to report it as no attention. Body \
             began: {:?}",
            body.chars().take(80).collect::<String>()
        ));
    }
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Wikimedia pageviews unparseable: {e}"))?;

    let items = v.get("items").and_then(|i| i.as_array()).ok_or_else(|| {
        "no `items` array in the Wikimedia response — refusing to read a missing \
             series as an absent one. A 404 here (a renamed article, or a window with \
             no data) arrives as a JSON body without `items`, and reporting that as \
             zero views would claim the public stopped caring."
            .to_string()
    })?;

    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let ts = it
            .get("timestamp")
            .and_then(|t| t.as_str())
            .ok_or_else(|| "a pageviews item had no `timestamp`".to_string())?;
        let views = it
            .get("views")
            .and_then(|x| x.as_u64())
            .ok_or_else(|| format!("pageviews item {ts} had no integer `views`"))?;
        // `YYYYMMDD00` -> `YYYY-MM-DD`.
        if ts.len() < 8 {
            return Err(format!(
                "pageviews timestamp {ts:?} is shorter than 8 chars"
            ));
        }
        out.push(DayViews {
            date: format!("{}-{}-{}", &ts[0..4], &ts[4..6], &ts[6..8]),
            views,
        });
    }
    out.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(out)
}

/// Fetch the daily series for one article.
pub fn fetch_series(
    f: &Fetcher,
    article: &str,
    start: &str,
    end: &str,
) -> Result<Vec<DayViews>, String> {
    let body = f.get(&pageviews_url(article, start, end), UA_WEB)?;
    parse_series(&body)
}

/// A trailing 7-day mean against the 7 days before it, in percent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttentionChange {
    pub label: String,
    /// Mean daily views over the most recent 7 complete days.
    pub recent_mean: f64,
    /// Mean daily views over the 7 days before that.
    pub prior_mean: f64,
    /// Percent change of `recent_mean` against `prior_mean`.
    pub pct: f64,
}

/// Compare the last 7 days against the previous 7, cancelling the weekday cycle.
///
/// Requires at least 14 points. Fewer is an **error**, not a partial comparison: a
/// 10-point series cannot supply a full prior week, and silently comparing 7 days
/// against 3 would produce a number dominated by whichever weekdays happened to be
/// present — the same class of defect as scoring a partial week as complete.
pub fn attention_change(label: &str, series: &[DayViews]) -> Result<AttentionChange, String> {
    const W: usize = 7;
    if series.len() < 2 * W {
        return Err(format!(
            "pageviews series for '{label}' has {} day(s); {} are needed to compare a \
             full week against the week before it",
            series.len(),
            2 * W
        ));
    }
    let n = series.len();
    let recent: f64 = series[n - W..].iter().map(|d| d.views as f64).sum::<f64>() / W as f64;
    let prior: f64 = series[n - 2 * W..n - W]
        .iter()
        .map(|d| d.views as f64)
        .sum::<f64>()
        / W as f64;
    if prior == 0.0 {
        return Err(format!(
            "the prior week for '{label}' summed to zero views, so no percentage change \
             can be formed"
        ));
    }
    Ok(AttentionChange {
        label: label.to_string(),
        recent_mean: recent,
        prior_mean: prior,
        pct: (recent / prior - 1.0) * 100.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_body_is_an_error_not_an_empty_series() {
        // The highest-consequence variant: a refusal that parses as "no activity"
        // reads as reassurance produced by a failure.
        let html = "<!DOCTYPE html><html><body>429 Too Many Requests</body></html>";
        let e = parse_series(html).unwrap_err();
        assert!(e.contains("non-JSON"), "got: {e}");
    }

    #[test]
    fn a_json_body_without_items_is_an_error_not_zero_views() {
        // A renamed article returns a JSON body with no `items`. Reading that as zero
        // would claim public attention collapsed.
        let e =
            parse_series(r#"{"type":"https://mediawiki.org/wiki/HyperSwitch/errors/not_found"}"#)
                .unwrap_err();
        assert!(e.contains("no `items` array"), "got: {e}");
    }

    #[test]
    fn a_present_but_empty_items_array_parses_to_an_empty_series() {
        // Distinct from the above: an explicit empty list IS a measurement of no
        // data for the window, and callers handle it as a gap by the length check.
        let v = parse_series(r#"{"items":[]}"#).unwrap();
        assert!(v.is_empty());
    }

    #[test]
    fn the_timestamp_is_normalised_to_a_date() {
        let v = parse_series(r#"{"items":[{"timestamp":"2026092400","views":1416}]}"#).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].date, "2026-09-24");
        assert_eq!(v[0].views, 1416);
    }

    #[test]
    fn the_series_is_returned_oldest_first_even_if_the_api_is_not() {
        let body = r#"{"items":[
            {"timestamp":"2026092400","views":3},
            {"timestamp":"2026092200","views":1},
            {"timestamp":"2026092300","views":2}]}"#;
        let v = parse_series(body).unwrap();
        assert_eq!(
            v.iter().map(|d| d.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-09-22", "2026-09-23", "2026-09-24"]
        );
    }

    fn series(views: &[u64]) -> Vec<DayViews> {
        views
            .iter()
            .enumerate()
            .map(|(i, v)| DayViews {
                date: format!("2026-09-{:02}", i + 1),
                views: *v,
            })
            .collect()
    }

    #[test]
    fn a_fewer_than_two_full_weeks_is_an_error_not_a_partial_comparison() {
        // 10 points cannot supply a full prior week; comparing 7 against 3 would be
        // dominated by whichever weekdays happened to be present.
        let e = attention_change("ai_bubble", &series(&[10; 10])).unwrap_err();
        assert!(e.contains("14 are needed"), "got: {e}");
        assert!(attention_change("ai_bubble", &series(&[10; 14])).is_ok());
    }

    #[test]
    fn the_weekday_cycle_cancels_because_a_full_week_is_compared() {
        // Two weeks of a pure weekly pattern, repeated. A per-day comparison would
        // report a large change; week-against-week must report none.
        let mut v = Vec::new();
        for _ in 0..2 {
            v.extend_from_slice(&[100, 100, 100, 100, 100, 40, 40]); // weekend dip
        }
        let c = attention_change("x", &series(&v)).unwrap();
        assert!(c.pct.abs() < 1e-9, "weekly cycle did not cancel: {}", c.pct);
    }

    #[test]
    fn a_real_doubling_is_reported_as_a_doubling() {
        let mut v = vec![10u64; 7];
        v.extend_from_slice(&[20u64; 7]);
        let c = attention_change("x", &series(&v)).unwrap();
        assert!((c.pct - 100.0).abs() < 1e-9, "got {}", c.pct);
        assert!((c.recent_mean - 20.0).abs() < 1e-9);
        assert!((c.prior_mean - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_zero_prior_week_is_an_error_rather_than_an_infinite_gain() {
        let mut v = vec![0u64; 7];
        v.extend_from_slice(&[5u64; 7]);
        let e = attention_change("x", &series(&v)).unwrap_err();
        assert!(e.contains("summed to zero"), "got: {e}");
    }

    #[test]
    fn the_url_targets_user_traffic_not_all_agents() {
        // `all-access` includes spiders, which would make an attention series partly a
        // measure of crawler traffic.
        let u = pageviews_url("AI_bubble", "20260825", "20260924");
        assert!(u.contains("/all-access/user/"), "got {u}");
        assert!(!u.contains("all-agents"), "got {u}");
        assert!(u.contains("/AI_bubble/daily/20260825/20260924"), "got {u}");
    }

    #[test]
    fn a_malformed_date_is_refused_rather_than_silently_compacted() {
        assert_eq!(compact_date("2026-09-24").unwrap(), "20260924");
        for bad in ["20260924", "2026-9-24", "2026/09/24", "", "2026-09-2"] {
            assert!(compact_date(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn the_tracked_set_is_non_empty_and_deduplicated() {
        let a = tracked_articles();
        assert!(!a.is_empty());
        let mut names: Vec<&str> = a.iter().map(|(l, _)| *l).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate article labels");
    }
}
