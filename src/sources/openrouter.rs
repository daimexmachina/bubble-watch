//! OpenRouter public token-volume feed — the project's only measurement of AI
//! compute *usage*.
//!
//! ## Why this source exists
//!
//! Every other indicator in this model measures cost (capex, funding gap,
//! depreciation), financing (spreads, issuance, leverage), price/froth
//! (valuation, volatility, breadth) or physical delivery (construction, grid).
//! **None measured whether the compute is actually used.** That is the gap this
//! closes: `capex_vs_cashflow` can see that spending outruns revenue, but it
//! cannot tell *demand failure* from *demand outrunning supply* — opposite
//! worlds that previously read identically.
//!
//! ## Verified behaviour (probed live 2026-09-20, keyless)
//!
//! | endpoint | status | size | returns |
//! |---|---|---|---|
//! | `api/frontend/v1/rankings/models` | 200 | 328,773 B | 573 rows: per-model, per-day prompt/completion token counts, 7 daily buckets |
//! | `api/frontend/v1/rankings/model-rankings-chart` | 200 | 23,935 B | 52 weekly points (~1 year), top-10 models/week |
//!
//! No key, no auth header, no browser. Measured window total: **128.3e12 tokens**
//! over 2026-09-13..2026-09-19.
//!
//! ## The honesty constraints
//!
//! * These are **undocumented frontend routes**, not a versioned public API. They
//!   may change without notice, which is exactly why every failure path here
//!   returns `Err` and degrades to a reported coverage gap rather than a number.
//! * OpenRouter is **one aggregator**, not the whole market. It is a lower bound
//!   on total AI usage, and the direction of that bias is stated wherever the
//!   reading is reported.
//! * A missing `data` array is an error, never an empty series: an empty series
//!   would read as "no demand" rather than "the source did not answer".

use crate::http::{Fetcher, UA_WEB};

/// Daily per-model token volume.
///
/// ⚠ **MEASURED TRAP: the "7-day window" is not seven complete days.** On
/// 2026-09-20 the payload held 573 rows but only **one date was fully populated**
/// (2026-09-19, 544 rows, 1.27e14 tokens). The other six dates carried 1-10 rows
/// each and 5-10 orders of magnitude fewer tokens (27 tokens on 2026-09-13).
/// Treating this as a daily series would produce wild nonsense; it is usable only
/// as a **latest snapshot**, and the code enforces that (`latest_complete_day`).
pub const RANKINGS_MODELS: &str = "https://openrouter.ai/api/frontend/v1/rankings/models";

/// Weekly history, top-10 models per week, ~1 year. Used for the rate of change,
/// because a single 7-day window cannot establish a trend.
pub const RANKINGS_CHART: &str =
    "https://openrouter.ai/api/frontend/v1/rankings/model-rankings-chart";

/// The model catalogue, which is what tells us whether a given model slug
/// publishes its weights (`hugging_face_id` present). Needed to compute the
/// open-weight share, which is the moat measurement.
pub const MODELS_CATALOGUE: &str = "https://openrouter.ai/api/v1/models";

/// Total tokens for one day, summed across every model and variant.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DailyTokens {
    /// `YYYY-MM-DD`. The source sends `"YYYY-MM-DD 00:00:00"`; the time part is
    /// discarded because the series is daily.
    pub date: String,
    pub total_tokens: u64,
    /// Tokens that went to models which publish their weights.
    pub open_weight_tokens: u64,
    /// Tokens whose model had NO catalogue entry, so it could not be classified.
    /// Kept separate because assuming either way would bias the share: treating
    /// them as closed depresses it, treating them as open inflates it. The live
    /// window carried 1.38% of tokens here, which bounds the share to
    /// 69.8%-71.2% rather than leaving it unknown.
    pub unclassified_tokens: u64,
    /// How many source rows contributed to this bucket.
    ///
    /// The completeness signal. A fully-populated day carries hundreds of rows; an
    /// incompletely back-filled one carries a handful. A demand reading built on a
    /// 3-row day would report a collapse that never happened — measured on
    /// 2026-09-20, six of the seven dates carried 1-10 rows against 544 for the one
    /// complete day.
    pub model_count: usize,
}

impl DailyTokens {
    /// Share of the day's tokens that went to open-weight models.
    ///
    /// This is the metric that answers "is intelligence commoditising", which
    /// `frontier_premium` was previously proxying with a price ratio.
    pub fn open_weight_share(&self) -> Option<f64> {
        if self.total_tokens == 0 {
            None
        } else {
            Some(self.open_weight_tokens as f64 / self.total_tokens as f64)
        }
    }

    /// The share if every unclassifiable token were treated as OPEN. Together
    /// with `open_weight_share()` this is an honest interval rather than a point
    /// estimate that hides how much of the payload could not be classified.
    pub fn open_weight_share_upper(&self) -> Option<f64> {
        if self.total_tokens == 0 {
            None
        } else {
            Some(
                (self
                    .open_weight_tokens
                    .saturating_add(self.unclassified_tokens)) as f64
                    / self.total_tokens as f64,
            )
        }
    }

    /// Fraction of the day's tokens that could not be classified at all.
    pub fn unclassified_share(&self) -> Option<f64> {
        if self.total_tokens == 0 {
            None
        } else {
            Some(self.unclassified_tokens as f64 / self.total_tokens as f64)
        }
    }
}

/// Parse the daily rankings payload into one record per date.
///
/// `open_slugs` is the set of model slugs known to publish weights (from
/// [`parse_open_weight_slugs`]). Passing an empty set is allowed and yields
/// `open_weight_tokens == 0` — the caller is responsible for not scoring a share
/// derived from an empty classification, and `open_weight_share()` returns the
/// arithmetic rather than a claim.
pub fn parse_token_volume(body: &str, index: &OpenWeightIndex) -> Result<Vec<DailyTokens>, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("rankings payload unparseable: {e}"))?;
    let rows = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "no `data` array in the rankings payload".to_string())?;

    // BTreeMap so the output is date-ordered and deterministic.
    let mut by: std::collections::BTreeMap<String, (u64, u64, u64, usize)> = Default::default();
    let mut dated = 0usize;
    for r in rows {
        let raw_date = r.get("date").and_then(|d| d.as_str()).unwrap_or("");
        if raw_date.is_empty() {
            continue;
        }
        let day = raw_date.split(' ').next().unwrap_or(raw_date).to_string();
        let p = r
            .get("total_prompt_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let c = r
            .get("total_completion_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let t = p.saturating_add(c);
        let slug = r
            .get("model_permaslug")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let e = by.entry(day).or_insert((0, 0, 0, 0));
        e.0 = e.0.saturating_add(t);
        e.3 += 1;
        match index.is_open(slug) {
            Some(true) => e.1 = e.1.saturating_add(t),
            None => e.2 = e.2.saturating_add(t),
            Some(false) => {}
        }
        dated += 1;
    }
    if dated == 0 {
        return Err("rankings payload contained no dated rows".to_string());
    }
    Ok(by
        .into_iter()
        .map(
            |(date, (total_tokens, open_weight_tokens, unclassified_tokens, model_count))| {
                DailyTokens {
                    date,
                    total_tokens,
                    open_weight_tokens,
                    unclassified_tokens,
                    model_count,
                }
            },
        )
        .collect())
}

/// Extract the model slugs that publish their weights, from the models
/// catalogue. A model is open-weight iff it carries a `hugging_face_id`.
///
/// ## The join key matters, and getting it wrong reads as a plausible number
///
/// The rankings feed keys models by **`canonical_slug`** (e.g.
/// `deepseek/deepseek-v4.1-flash-20260910`), while the catalogue's `id` drops the
/// date suffix (`deepseek/deepseek-v4.1-flash`). Joining on `id` alone matched
/// almost nothing and produced an open-weight share of **2.3%** where the truth
/// is **~70%** — a wrong answer that looked like a real measurement. Both keys
/// are therefore indexed.
///
/// ## Unmatched slugs are counted, not assumed closed
///
/// 165 of 499 slugs in the live window had no catalogue entry. They carry only
/// **1.38%** of tokens, so the share is bounded rather than guessed:
/// **69.8% (unmatched closed) … 71.2% (unmatched open)**. The reading reports
/// that interval instead of one number.
///
/// Returns an error if the catalogue has no `data` array: an empty set would
/// silently make every model look closed and drive the share to zero, which is a
/// fabricated reading rather than a missing one.
pub fn parse_open_weight_classification(catalogue_body: &str) -> Result<OpenWeightIndex, String> {
    let v: serde_json::Value = serde_json::from_str(catalogue_body)
        .map_err(|e| format!("catalogue payload unparseable: {e}"))?;
    let rows = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "no `data` array in the models catalogue".to_string())?;

    let mut by_id: std::collections::BTreeMap<String, bool> = Default::default();
    let mut by_canonical: std::collections::BTreeMap<String, bool> = Default::default();
    let mut n_open = 0usize;
    for r in rows {
        let is_open = r
            .get("hugging_face_id")
            .map(|h| !h.is_null())
            .unwrap_or(false);
        if is_open {
            n_open += 1;
        }
        if let Some(id) = r.get("id").and_then(|x| x.as_str()) {
            by_id.insert(id.to_string(), is_open);
        }
        if let Some(cs) = r.get("canonical_slug").and_then(|x| x.as_str()) {
            by_canonical.insert(cs.to_string(), is_open);
        }
    }
    if by_id.is_empty() {
        return Err("models catalogue contained no identifiable models".to_string());
    }
    if n_open == 0 {
        return Err(
            "models catalogue marked NO model as open-weight — refusing to report a \
             zero open-weight share from an unclassifiable catalogue"
                .to_string(),
        );
    }
    Ok(OpenWeightIndex {
        by_id,
        by_canonical,
    })
}

/// Index of which models publish their weights, keyed both ways so a slug from
/// either feed resolves.
#[derive(Debug, Clone, Default)]
pub struct OpenWeightIndex {
    by_id: std::collections::BTreeMap<String, bool>,
    by_canonical: std::collections::BTreeMap<String, bool>,
}

impl OpenWeightIndex {
    /// `None` when the slug is unknown — callers must count these, never assume
    /// they are closed.
    pub fn is_open(&self, slug: &str) -> Option<bool> {
        self.by_canonical
            .get(slug)
            .or_else(|| self.by_id.get(slug))
            .copied()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Parse the weekly history chart into (week_start, tokens) pairs.
///
/// The chart is the only endpoint here with enough history to compute a rate of
/// change, which is the signal that matters: a level alone cannot show
/// deceleration.
pub fn parse_weekly_chart(body: &str) -> Result<Vec<DailyTokens>, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("chart payload unparseable: {e}"))?;

    // MEASURED SHAPE, not the guessed one. The endpoint nests twice:
    //   {"data": {"data": [ {"x": "2025-09-22", "ys": {"<model>": <tokens>}}, ... ],
    //             "cachedAt": <ms>}}
    // An earlier version expected `data` to be the array directly and failed on the
    // live payload with "no `data` array in the chart payload" while every fixture
    // test passed. The fixtures had been written to match the guess.
    let inner = v
        .get("data")
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| {
            "no `data.data` array in the chart payload (the endpoint nests its rows \
             one level deeper than the daily rankings feed)"
                .to_string()
        })?;

    let mut out = Vec::new();
    for r in inner {
        let wk = r.get("x").and_then(|x| x.as_str()).unwrap_or("");
        if wk.is_empty() {
            continue;
        }
        // `ys` maps model -> tokens for that week; the week's total is the sum
        // across every model the chart chose to include (it shows the top ten).
        let total: u64 = r
            .get("ys")
            .and_then(|y| y.as_object())
            .map(|m| {
                m.values()
                    .filter_map(|x| x.as_u64())
                    .fold(0u64, |a, b| a.saturating_add(b))
            })
            .unwrap_or(0);
        let day = wk.split(' ').next().unwrap_or(wk).to_string();
        out.push(DailyTokens {
            date: day,
            total_tokens: total,
            open_weight_tokens: 0,
            unclassified_tokens: 0,
            // The chart lists the top ten models per week, so the count is a
            // "top-N present" marker, not a completeness measure like the daily
            // feed's row count. The partial-week guard is separate.
            model_count: r
                .get("ys")
                .and_then(|y| y.as_object())
                .map(|m| m.len())
                .unwrap_or(0),
        });
    }
    if out.is_empty() {
        return Err("chart payload contained no dated rows".to_string());
    }
    out.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(out)
}

/// The most recent day whose row count indicates a COMPLETE day.
///
/// The daily feed back-fills incompletely: only one date typically carries the
/// full model set. Returning the last date regardless would read a stub day as a
/// collapse in demand. `min_models` is the floor below which a date is treated as
/// incomplete; pass the observed complete-day row count, or a defensible fraction
/// of it.
pub fn latest_complete_day(days: &[DailyTokens], min_models: usize) -> Option<&DailyTokens> {
    // `parse_token_volume` returns date-ascending order.
    days.iter().rev().find(|d| d.model_count >= min_models)
}

/// Drop a trailing week that is implausibly small against its predecessor.
///
/// The live chart's newest week is routinely PARTIAL, and a partial week is not a
/// fall in usage — it is an unfinished week.
///
/// ## Why this is decided by the CALENDAR, not by magnitude
///
/// The first version of this guard compared the newest week's token count to the
/// previous week's and dropped it only when the ratio exceeded a threshold (10.0 at
/// the call site). That looked reasonable against the artefact it was built for — a
/// 5.8e10 week against 1.29e14, a factor of ~2,200 — but it **could not fire for a
/// week that was merely young**, which is the common case. Measured 2026-09-22: the
/// trailing week held 3.88e13 against 1.29e14, a ratio of only 3.32x, so the guard
/// passed it through. The arithmetic shows why the threshold was unworkable: firing
/// at ratio > 10 requires fewer than 0.7 of 7 days to have elapsed, so it was
/// effectively dead code.
///
/// The consequence was not cosmetic. Scoring a 2-day-old week as if it were a
/// finished week turned a genuine +176% 13-week growth into -17%, which moved this
/// indicator from 26.8 to 89.4 stress — a **62.6-point error** on a 10-weight
/// indicator, and it read as demand COLLAPSE when demand had grown.
///
/// So the test is now what it always should have been: how much of the week has
/// actually happened. A trailing week whose label is less than `min_elapsed_days`
/// before `today` is in progress and is dropped. Week labels are the week's START
/// date (confirmed: the 2026-09-21 row held ~28.6% of a full week's tokens on
/// 2026-09-22, i.e. 2 of 7 days).
///
/// Returns the series with the trailing in-progress week removed, or the series
/// unchanged when the last week is complete.
pub fn drop_partial_trailing_week(
    weeks: &[DailyTokens],
    today: &str,
    min_elapsed_days: i64,
) -> Vec<DailyTokens> {
    if weeks.len() < 2 {
        return weeks.to_vec();
    }
    let Some(last_label) =
        chrono::NaiveDate::parse_from_str(&weeks[weeks.len() - 1].date, "%Y-%m-%d").ok()
    else {
        // An unparseable label is not evidence of completeness. Keep the series
        // whole rather than silently discarding a week on a parse failure.
        return weeks.to_vec();
    };
    let Some(today) = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok() else {
        return weeks.to_vec();
    };
    let elapsed_days = (today - last_label).num_days();
    if elapsed_days < min_elapsed_days {
        weeks[..weeks.len() - 1].to_vec()
    } else {
        weeks.to_vec()
    }
}

/// Days a week must have been running before its row counts as complete.
///
/// A labelled week START needs 7 days to finish, so anything under 7 is in
/// progress. The threshold is 7 rather than 6 because a week is complete only once
/// its seventh day has elapsed; a run landing on the boundary should drop the week
/// rather than half-count it.
pub const WEEK_COMPLETE_DAYS: i64 = 7;

/// Fetch and parse the daily token volume plus the open-weight classification.
pub fn fetch_daily(f: &Fetcher) -> Result<Vec<DailyTokens>, String> {
    let cat = f.get(MODELS_CATALOGUE, UA_WEB)?;
    let index = parse_open_weight_classification(&cat)?;
    let body = f.get(RANKINGS_MODELS, UA_WEB)?;
    parse_token_volume(&body, &index)
}

/// Fetch and parse the weekly history.
pub fn fetch_weekly(f: &Fetcher) -> Result<Vec<DailyTokens>, String> {
    let body = f.get(RANKINGS_CHART, UA_WEB)?;
    parse_weekly_chart(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(entries: &[(&str, bool, Option<&str>)]) -> OpenWeightIndex {
        // helper: (slug, is_open, optional canonical_slug)
        let mut by_id = std::collections::BTreeMap::new();
        let mut by_canonical = std::collections::BTreeMap::new();
        for (slug, open, canon) in entries {
            by_id.insert(slug.to_string(), *open);
            if let Some(c) = canon {
                by_canonical.insert(c.to_string(), *open);
            }
        }
        OpenWeightIndex {
            by_id,
            by_canonical,
        }
    }

    #[test]
    fn a_missing_data_array_is_an_error_not_an_empty_series() {
        // Every source in this project makes this distinction; an empty vec would
        // silently read as "no demand" rather than "the source did not answer".
        let e = parse_token_volume(r#"{"unexpected": true}"#, &idx(&[])).unwrap_err();
        assert!(
            e.contains("data"),
            "error must name what was missing: {}",
            e
        );
    }

    #[test]
    fn an_undated_payload_is_an_error_not_a_zero_series() {
        let body = r#"{"data":[{"model_permaslug":"a","total_prompt_tokens":10}]}"#;
        let e = parse_token_volume(body, &idx(&[])).unwrap_err();
        assert!(
            e.contains("dated"),
            "error must say what was missing: {}",
            e
        );
    }

    #[test]
    fn tokens_are_summed_across_models_and_variants_for_a_date() {
        let body = r#"{"data":[
          {"date":"2026-09-19 00:00:00","model_permaslug":"a","variant":"standard",
           "total_prompt_tokens":100,"total_completion_tokens":10},
          {"date":"2026-09-19 00:00:00","model_permaslug":"b","variant":"free",
           "total_prompt_tokens":50,"total_completion_tokens":5}
        ]}"#;
        let v = parse_token_volume(body, &idx(&[("a", true, None), ("b", false, None)])).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].total_tokens, 165);
        assert_eq!(v[0].date, "2026-09-19");
    }

    #[test]
    fn open_weight_share_counts_both_prompt_and_completion_tokens() {
        // The numerator must include BOTH token kinds. An earlier draft expected
        // 100/165 because it counted only prompt tokens on the open side; the
        // value is 110/165.
        let body = r#"{"data":[
          {"date":"2026-09-19 00:00:00","model_permaslug":"open/x","variant":"standard",
           "total_prompt_tokens":100,"total_completion_tokens":10},
          {"date":"2026-09-19 00:00:00","model_permaslug":"closed/y","variant":"standard",
           "total_prompt_tokens":50,"total_completion_tokens":5}
        ]}"#;
        let v = parse_token_volume(
            body,
            &idx(&[("open/x", true, None), ("closed/y", false, None)]),
        )
        .unwrap();
        assert_eq!(v[0].open_weight_tokens, 110);
        assert_eq!(v[0].unclassified_tokens, 0);
        let share = v[0].open_weight_share().unwrap();
        assert!((share - 110.0 / 165.0).abs() < 1e-12, "got {}", share);
        assert!((v[0].open_weight_share_upper().unwrap() - share).abs() < 1e-12);
    }

    #[test]
    fn the_share_is_resolved_through_canonical_slug_not_only_id() {
        // THE LIVE BUG. The rankings feed keys by `canonical_slug`
        // (with a date suffix); the catalogue's `id` omits it. Joining on `id`
        // alone classified almost nothing and reported an open-weight share of
        // 2.3% where the truth is ~70% — a wrong number that looked measured.
        let body = r#"{"data":[
          {"date":"2026-09-19 00:00:00","model_permaslug":"deepseek/v4.1-20260910",
           "total_prompt_tokens":100,"total_completion_tokens":0}
        ]}"#;
        let i = idx(&[("deepseek/v4.1", true, Some("deepseek/v4.1-20260910"))]);
        let v = parse_token_volume(body, &i).unwrap();
        assert_eq!(v[0].open_weight_tokens, 100, "canonical_slug must resolve");
        assert_eq!(v[0].unclassified_tokens, 0);
    }

    #[test]
    fn an_unmatched_slug_is_counted_as_unclassified_never_assumed_closed() {
        // Assuming closed would silently depress the share; assuming open would
        // inflate it. Neither is a measurement, so the token is bucketed.
        let body = r#"{"data":[
          {"date":"2026-09-19 00:00:00","model_permaslug":"ghost/x",
           "total_prompt_tokens":100,"total_completion_tokens":0},
          {"date":"2026-09-19 00:00:00","model_permaslug":"open/y",
           "total_prompt_tokens":100,"total_completion_tokens":0}
        ]}"#;
        let v = parse_token_volume(body, &idx(&[("open/y", true, None)])).unwrap();
        assert_eq!(v[0].unclassified_tokens, 100);
        assert_eq!(v[0].open_weight_tokens, 100);
        // The interval must bracket the point estimate.
        let lo = v[0].open_weight_share().unwrap();
        let hi = v[0].open_weight_share_upper().unwrap();
        assert!((lo - 0.5).abs() < 1e-12, "lower bound got {lo}");
        assert!((hi - 1.0).abs() < 1e-12, "upper bound got {hi}");
        assert!((v[0].unclassified_share().unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn an_empty_or_unclassifiable_catalogue_is_refused() {
        let e = parse_open_weight_classification(r#"{"data":[]}"#).unwrap_err();
        assert!(e.contains("no identifiable models"), "got {}", e);
        let e2 = parse_open_weight_classification(r#"{"nope":1}"#).unwrap_err();
        assert!(e2.contains("data"), "must name the missing key: {}", e2);
        // A catalogue where NOTHING is open would drive the share to zero, which
        // is a fabricated reading rather than a missing one.
        let e3 =
            parse_open_weight_classification(r#"{"data":[{"id":"a","hugging_face_id":null}]}"#)
                .unwrap_err();
        assert!(e3.contains("NO model"), "got {}", e3);
    }

    #[test]
    fn a_zero_token_day_has_no_share_rather_than_a_zero_share() {
        let d = DailyTokens {
            date: "2026-09-19".into(),
            total_tokens: 0,
            open_weight_tokens: 0,
            unclassified_tokens: 0,
            model_count: 0,
        };
        assert!(d.open_weight_share().is_none());
        assert!(d.open_weight_share_upper().is_none());
        assert!(d.unclassified_share().is_none());
    }

    #[test]
    fn token_sums_saturate_rather_than_overflow() {
        let body = format!(
            r#"{{"data":[
              {{"date":"2026-09-19 00:00:00","model_permaslug":"a","total_prompt_tokens":{m},"total_completion_tokens":{m}}},
              {{"date":"2026-09-19 00:00:00","model_permaslug":"b","total_prompt_tokens":{m},"total_completion_tokens":{m}}}
            ]}}"#,
            m = u64::MAX
        );
        let v = parse_token_volume(&body, &idx(&[])).unwrap();
        assert_eq!(v[0].total_tokens, u64::MAX, "must saturate, not wrap");
    }

    #[test]
    fn the_weekly_chart_parses_its_ACTUAL_nested_shape() {
        // The live payload nests: data.data[] with {x: week, ys: {model: tokens}}.
        // An earlier version expected `data` to BE the array; every fixture test
        // passed because the fixtures had been written to match the guess, and it
        // only failed against the live endpoint.
        let body = r#"{"data":{"cachedAt":1789949159135,"data":[
          {"x":"2025-09-22","ys":{"a/b":100,"c/d":50}},
          {"x":"2025-09-15","ys":{"a/b":40}}
        ]}}"#;
        let v = parse_weekly_chart(body).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].date, "2025-09-15", "oldest first");
        assert_eq!(v[1].total_tokens, 150, "ys summed across models");
    }

    #[test]
    fn the_flat_shape_is_rejected_with_an_explanation() {
        // Guards the exact regression: rows directly under `data` must NOT be
        // silently accepted, or the shape change would go unnoticed.
        let e =
            parse_weekly_chart(r#"{"data":[{"x":"2025-09-22","total_tokens":1}]}"#).unwrap_err();
        assert!(e.contains("data.data"), "must name the real path: {}", e);
    }

    #[test]
    fn an_incompletely_backfilled_day_is_not_returned_as_the_latest() {
        // THE LIVE TRAP. Six of seven dates in the real payload carried 1-10 rows
        // while one carried 544. Returning the newest date regardless would read a
        // stub day as a collapse in demand.
        let body = r#"{"data":[
          {"date":"2026-09-18 00:00:00","model_permaslug":"a","total_prompt_tokens":5},
          {"date":"2026-09-19 00:00:00","model_permaslug":"a","total_prompt_tokens":100},
          {"date":"2026-09-19 00:00:00","model_permaslug":"b","total_prompt_tokens":100},
          {"date":"2026-09-19 00:00:00","model_permaslug":"c","total_prompt_tokens":100}
        ]}"#;
        let v = parse_token_volume(body, &idx(&[])).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].date, "2026-09-19");
        assert_eq!(v[1].model_count, 3);
        assert_eq!(v[0].model_count, 1);

        let latest = latest_complete_day(&v, 3).expect("a complete day exists");
        assert_eq!(latest.date, "2026-09-19");
        assert_eq!(latest.total_tokens, 300);

        // With a higher floor, NO day qualifies — and it must say so rather than
        // falling back to the newest stub.
        assert!(latest_complete_day(&v, 600).is_none());
    }

    #[test]
    fn a_partial_trailing_week_is_dropped_not_read_as_a_collapse() {
        // THE OTHER LIVE TRAP. The newest week read 5.8e10 against ~1.29e14 for the
        // week before — a ~2,200x artefact of the week being unfinished. Undetected,
        // a growth calculation on this series reported "0.0x over the window".
        let body = r#"{"data":{"data":[
          {"x":"2026-09-07","ys":{"a":100,"b":100}},
          {"x":"2026-09-14","ys":{"a":120,"b":100}},
          {"x":"2026-09-21","ys":{"a":1}}
        ]}}"#;
        let v = parse_weekly_chart(body).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(
            v[2].total_tokens, 1,
            "the partial week is present in the parse"
        );

        // Today is 2026-09-23: the 2026-09-21 week has run 2 of its 7 days.
        let trimmed = drop_partial_trailing_week(&v, "2026-09-23", WEEK_COMPLETE_DAYS);
        assert_eq!(trimmed.len(), 2, "the partial week must be dropped");
        assert_eq!(trimmed[1].date, "2026-09-14");
    }

    #[test]
    fn a_COMPLETE_trailing_week_is_kept() {
        // The guard must not discard real data. 2026-09-14 has run its full 7 days
        // by 2026-09-21, so the week is finished and kept.
        let body = r#"{"data":{"data":[
          {"x":"2026-09-07","ys":{"a":100}},
          {"x":"2026-09-14","ys":{"a":120}}
        ]}}"#;
        let v = parse_weekly_chart(body).unwrap();
        assert_eq!(
            drop_partial_trailing_week(&v, "2026-09-21", WEEK_COMPLETE_DAYS).len(),
            2,
            "a week whose 7 days have elapsed is complete and must be kept"
        );
    }

    #[test]
    fn a_MERELY_YOUNG_week_is_dropped_even_though_it_is_not_small() {
        // THE BUG THIS REPLACES. The magnitude guard required the trailing week to
        // be >10x smaller than the previous one, which can only happen with fewer
        // than 0.7 of 7 days elapsed — so a week that was merely 2 days old sailed
        // through. Measured live 2026-09-22: 3.88e13 against 1.29e14, a ratio of
        // only 3.32x. Reading that as a finished week reported -17% deceleration
        // when 13-week growth was +176%, moving the indicator 62.6 stress points.
        let body = r#"{"data":{"data":[
          {"x":"2026-09-07","ys":{"a":500}},
          {"x":"2026-09-14","ys":{"a":1290}},
          {"x":"2026-09-21","ys":{"a":388}}
        ]}}"#;
        let v = parse_weekly_chart(body).unwrap();
        // 3.32x smaller — well inside the old ratio threshold of 10.0, so the old
        // guard KEPT this. It is 2 days old, so it must be dropped.
        assert!(
            1290.0 / 388.0 < 10.0,
            "precondition: this week is too big for the OLD magnitude guard to catch"
        );
        let trimmed = drop_partial_trailing_week(&v, "2026-09-23", WEEK_COMPLETE_DAYS);
        assert_eq!(
            trimmed.len(),
            2,
            "a 2-day-old week must be dropped however large it looks"
        );
        assert_eq!(trimmed[1].date, "2026-09-14");
    }

    #[test]
    fn a_single_week_is_left_alone_by_the_partial_guard() {
        // With one point there is nothing to compare against, so the guard must
        // not delete it — the caller decides what a lone point means.
        let body = r#"{"data":{"data":[{"x":"2026-09-14","ys":{"a":100}}]}}"#;
        let v = parse_weekly_chart(body).unwrap();
        assert_eq!(
            drop_partial_trailing_week(&v, "2026-09-23", WEEK_COMPLETE_DAYS).len(),
            1
        );
    }

    #[test]
    fn an_unparseable_body_says_so_rather_than_panicking() {
        for bad in ["", "not json", "[]"] {
            assert!(parse_token_volume(bad, &idx(&[])).is_err());
            assert!(parse_weekly_chart(bad).is_err());
            assert!(parse_open_weight_classification(bad).is_err());
        }
    }
}
