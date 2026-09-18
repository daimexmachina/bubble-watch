//! Core data model. Every number that reaches the report carries provenance or
//! is explicitly marked unavailable — there is no third state.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Where a number came from. Attached to every scored value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Provenance {
    pub source: String,
    /// Exact endpoint or series id, so a reader can re-fetch the same datum.
    pub endpoint: String,
    /// Date the datum refers to.
    pub as_of: String,
    /// Date/time we retrieved it.
    pub retrieved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Series {
    pub provenance: Provenance,
    pub points: Vec<Point>,
}

impl Series {
    pub fn last(&self) -> Option<&Point> {
        self.points.last()
    }
    pub fn latest_value(&self) -> Option<f64> {
        self.last().map(|p| p.value)
    }
    pub fn values(&self) -> Vec<f64> {
        self.points.iter().map(|p| p.value).collect()
    }
}

/// A single fact pulled from an SEC XBRL company-concept endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgarFact {
    pub tag: String,
    pub start: String,
    pub end: String,
    pub val: f64,
    pub form: String,
    pub filed: String,
    /// Duration in days, used to distinguish quarterly from annual facts.
    pub days: i64,
}

/// The set of fundamentals needed for one company in the cohort.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CompanyFacts {
    pub cik: String,
    pub name: String,
    /// Quarterly capital-expenditure facts, newest last.
    pub capex: Vec<EdgarFact>,
    pub cfo: Vec<EdgarFact>,
    pub revenue: Vec<EdgarFact>,
    pub shares: Vec<EdgarFact>,
    pub debt: Vec<EdgarFact>,
    /// Cash paid to repurchase common stock (cash-flow statement).
    pub buyback: Vec<EdgarFact>,
    /// Cash received from issuing common stock. Genuinely absent for filers that
    /// do not issue equity (AMZN), which is a gap and not a zero.
    pub issuance: Vec<EdgarFact>,
    /// Operating lease liability (point in time).
    pub lease: Vec<EdgarFact>,
    /// Unconditional purchase obligations. Absent for filers that do not disclose
    /// them (MSFT), which is a gap and never a zero.
    pub purchase_obligation: Vec<EdgarFact>,
    /// Remaining performance obligation: contracted revenue not yet recognised.
    /// Absent entirely for META, which is a gap and never a decline.
    pub rpo: Vec<EdgarFact>,
    /// Contract liability / deferred revenue: cash billed but not yet recognised.
    /// The contrast with `rpo` is what reveals whether backlog converts to cash.
    pub deferred_revenue: Vec<EdgarFact>,
    /// Long-term debt maturing within 1, 2 and 3 years, indexed 0..2.
    pub debt_due: [Vec<EdgarFact>; 3],
    /// Long-term debt maturing after five years.
    pub debt_due_after: Vec<EdgarFact>,
}

impl CompanyFacts {
    /// Trailing-twelve-month sum of a quarterly fact series.
    ///
    /// Sums the four most recent non-overlapping quarters. Falls back to the
    /// most recent annual (10-K) fact when there are not yet four quarters —
    /// and reports which basis was used, so the caller can be honest about it.
    pub fn ttm(facts: &[EdgarFact]) -> Option<(f64, TtmBasis)> {
        // Prefer true quarter-length facts (~80-100 days).
        let mut quarters: Vec<&EdgarFact> = facts
            .iter()
            .filter(|f| (80..=100).contains(&f.days))
            .collect();
        quarters.sort_by(|a, b| a.end.cmp(&b.end));

        // Walk backwards taking non-overlapping quarters.
        let mut chosen: Vec<&EdgarFact> = Vec::new();
        let mut cursor: Option<String> = None;
        for f in quarters.iter().rev() {
            match &cursor {
                None => chosen.push(f),
                Some(c) => {
                    if f.end.as_str() < c.as_str() {
                        chosen.push(f);
                    }
                }
            }
            cursor = Some(f.end.clone());
        }
        if chosen.len() >= 4 {
            let sum: f64 = chosen.iter().take(4).map(|f| f.val).sum();
            return Some((sum, TtmBasis::FourQuarters));
        }

        // Fall back to the most recent annual fact.
        let mut annuals: Vec<&EdgarFact> = facts
            .iter()
            .filter(|f| (350..=380).contains(&f.days))
            .collect();
        annuals.sort_by(|a, b| a.end.cmp(&b.end));
        annuals.last().map(|f| (f.val, TtmBasis::AnnualFallback))
    }

    /// Trailing-twelve-month sum as of `offset_q` quarters back from the latest.
    ///
    /// `offset_q = 0` is the ordinary TTM. `offset_q = 4` gives the TTM a year
    /// earlier, which is what makes a year-over-year comparison of aggregates
    /// possible without refetching anything. Returns None when there is not enough
    /// history rather than summing a shorter window and mislabelling it.
    /// Trailing-twelve-month sum as of `offset_q` quarters back from the latest.
    ///
    /// `offset_q = 0` is the ordinary TTM. `offset_q = 4` gives the TTM a year
    /// earlier, which is what makes a year-over-year comparison of aggregates
    /// possible without refetching anything.
    ///
    /// Selection matches `ttm` exactly: walk backwards taking NON-OVERLAPPING
    /// quarters, detected by comparing each candidate's END against the running
    /// cursor's START. A plain end-to-end contiguity test is wrong here because
    /// filers such as MSFT report BOTH quarterly and cumulative year-to-date
    /// figures, so two facts legitimately sit six months apart in a series that is
    /// still a valid quarterly sequence. Rejecting those windows made the
    /// year-over-year comparison unavailable for half the cohort.
    ///
    /// Returns None when there genuinely is not enough history, rather than summing
    /// a shorter window and mislabelling it.
    pub fn ttm_at_offset(facts: &[EdgarFact], offset_q: usize) -> Option<(f64, TtmBasis)> {
        let mut quarters: Vec<&EdgarFact> = facts
            .iter()
            .filter(|f| (80..=100).contains(&f.days))
            .collect();
        quarters.sort_by(|a, b| a.end.cmp(&b.end));

        // Take non-overlapping quarters walking backwards, skipping the first
        // `offset_q` of them.
        let mut chosen: Vec<&EdgarFact> = Vec::new();
        let mut cursor_start: Option<String> = None;
        for f in quarters.iter().rev() {
            match &cursor_start {
                None => {
                    // The most recent quarter: always eligible.
                    if offset_q == 0 || chosen.len() < offset_q {
                        chosen.push(f);
                        cursor_start = Some(f.start.clone());
                    }
                }
                Some(cs) => {
                    // Non-overlapping means it ENDS before the cursor STARTS. Using
                    // the start rather than the end is what lets a quarterly fact
                    // follow a cumulative one correctly.
                    if f.end.as_str() < cs.as_str() && chosen.len() < offset_q + 4 {
                        chosen.push(f);
                        cursor_start = Some(f.start.clone());
                    }
                }
            }
            if chosen.len() >= offset_q + 4 {
                break;
            }
        }
        if chosen.len() < offset_q + 4 {
            return None;
        }
        Some((chosen.iter().map(|f| f.val).sum(), TtmBasis::FourQuarters))
    }

    /// Most recent fact of any duration, for point-in-time series like shares.
    pub fn latest(facts: &[EdgarFact]) -> Option<&EdgarFact> {
        facts.iter().max_by(|a, b| a.end.cmp(&b.end))
    }

    /// The balance-sheet value of a point-in-time series as of `as_of`, refusing
    /// stale or implausible observations.
    ///
    /// Why this exists as a named function rather than a `latest()` call: ORCL
    /// publishes `LongTermDebt` as a single stale fact ending 2022-05-31 with a
    /// value of 0.0. Taking the latest fact would report Oracle — the most
    /// leveraged company in the cohort — as carrying no debt.
    ///
    /// A value is rejected when:
    ///   * it is older than `max_age_days` relative to `as_of` (a stale series is
    ///     not an observation of the present); or
    ///   * it is exactly zero (no large filer has zero long-term debt, so a zero
    ///     almost always means a different concept or a placeholder).
    ///
    /// Rejection returns `None`, which the caller must render as unavailable — not
    /// as zero. Returns the fact and a reason string when rejected, so the caller
    /// can say WHY rather than silently dropping the filer.
    pub fn point_in_time<'a>(
        facts: &'a [EdgarFact],
        as_of: &str,
        max_age_days: i64,
    ) -> Result<&'a EdgarFact, String> {
        let Some(f) = facts.iter().max_by(|a, b| a.end.cmp(&b.end)) else {
            return Err("no facts reported under this concept".into());
        };
        let age = crate::sources::edgar::days_between(&f.end, as_of);
        if age > max_age_days {
            return Err(format!(
                "latest reported value is stale: {} is {} days before {}",
                f.end, age, as_of
            ));
        }
        if f.val == 0.0 {
            return Err(format!(
                "latest reported value is exactly zero at {}, which for this concept indicates a \
                 wrong tag rather than an absence of debt",
                f.end
            ));
        }
        Ok(f)
    }

    /// Why a stock-based series is unusable, as a machine-readable code plus a
    /// human reason.
    ///
    /// Every variant here was observed live on the real cohort. A naive
    /// implementation emits a number for all of them and produces false positives:
    /// on gross PP&E alone, 4 of 9 companies fail.
    pub fn series_health(
        facts: &[EdgarFact],
        as_of: &str,
        min_annual_facts: usize,
        max_gap_days: i64,
        max_stale_days: i64,
        max_single_period_growth: f64,
    ) -> Result<(), String> {
        if facts.len() < min_annual_facts {
            return Err(format!(
                "THIN: only {} observation(s), fewer than the {} required",
                facts.len(),
                min_annual_facts
            ));
        }
        let mut xs: Vec<&EdgarFact> = facts.iter().collect();
        xs.sort_by(|a, b| a.end.cmp(&b.end));

        // Recency. GOOGL's gross PP&E is 20 months stale and META's 92 months.
        let latest = xs.last().unwrap();
        let stale = crate::sources::edgar::days_between(&latest.end, as_of);
        if stale > max_stale_days {
            return Err(format!(
                "STALE: latest observation {} is {} days before {}, beyond the {} day limit",
                latest.end, stale, as_of, max_stale_days
            ));
        }

        // Continuity. AMZN's gross PP&E has a 1827-day hole, which sorted-series
        // arithmetic silently bridges as though nothing happened.
        for w in xs.windows(2) {
            let gap = crate::sources::edgar::days_between(&w[0].end, &w[1].end);
            if gap > max_gap_days {
                return Err(format!(
                    "GAP: {} days between {} and {}, beyond the {} day limit",
                    gap, w[0].end, w[1].end, max_gap_days
                ));
            }
        }

        // Definition stability. AMZN's 3.29x single-year jump is a tag definition
        // swap (PP&E-only to PP&E-including-finance-lease-ROU), not a build-out.
        for w in xs.windows(2) {
            if w[0].val > 0.0 {
                let growth = w[1].val / w[0].val;
                if growth > max_single_period_growth {
                    return Err(format!(
                        "DEFINITION_SWAP: {:.2}x growth from {} to {} exceeds the {:.2}x ceiling, \
                         which indicates the tag changed meaning rather than the business changing \
                         size",
                        growth, w[0].end, w[1].end, max_single_period_growth
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TtmBasis {
    FourQuarters,
    AnnualFallback,
}

/// Raw material collected from the network. Pure input to scoring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Observations {
    /// Keyed by symbol, e.g. "SPX_5y_mo".
    pub yahoo: BTreeMap<String, Series>,
    /// Keyed by FRED series id.
    pub fred: BTreeMap<String, Series>,
    /// Which transport supplied each FRED series, so the report can say whether
    /// a reading came from the keyed API or the anonymous CSV fallback.
    #[serde(default)]
    pub fred_transports: BTreeMap<String, crate::sources::fred::Transport>,
    /// Keyed by ticker.
    pub edgar: BTreeMap<String, CompanyFacts>,
    /// EDGAR AI-mention census: (form type, year, filing count). A census, not a
    /// sample, and therefore usable as a hype measure when most alternatives are
    /// samples of unclear provenance.
    #[serde(default)]
    pub ai_census: Vec<(String, u32, u64)>,
    /// Provenance for the census, so the indicator can cite it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_census_provenance: Option<Provenance>,
    /// Cancellation ratio from EIA-860M: cancelled / (planned + cancelled), as a
    /// percentage. Stored as the formatted value the source produced so the indicator
    /// cannot silently reformat it.
    #[serde(default)]
    pub eia_ratio: Option<String>,
    pub failures: Vec<SourceFailure>,
    pub retrieved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceFailure {
    pub source: String,
    pub endpoint: String,
    pub reason: String,
}

/// The outcome of evaluating one indicator. Two states only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reading {
    Scored {
        /// 0 = calm, 100 = historical extreme. Higher = more bubble-like.
        stress: f64,
        /// The underlying primitive, in `unit`.
        value: f64,
        unit: String,
        detail: String,
        provenance: Provenance,
    },
    Unavailable {
        reason: String,
    },
}

impl Reading {
    pub fn stress(&self) -> Option<f64> {
        match self {
            Reading::Scored { stress, .. } => Some(*stress),
            Reading::Unavailable { .. } => None,
        }
    }
    pub fn is_available(&self) -> bool {
        matches!(self, Reading::Scored { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorReading {
    pub id: String,
    pub label: String,
    /// Configured weight, before renormalization.
    pub weight: f64,
    pub rationale: String,
    pub reading: Reading,
    /// Weighted contribution to the composite, AFTER renormalization.
    /// `None` when the indicator is unavailable or has zero weight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contribution: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalogWindow {
    pub method: String,
    pub is_probability: bool,
    pub analogs: Vec<String>,
    pub derivation: String,
    pub caveat: String,
    /// Coarse range in months, from the band matching the composite.
    pub range_months: Option<[f64; 2]>,
    pub band: Option<String>,
    pub band_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnavailableItem {
    pub id: String,
    pub label: String,
    pub weight: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataQuality {
    pub total_weight: f64,
    pub available_weight: f64,
    pub coverage: f64,
    pub unavailable: Vec<UnavailableItem>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceHealth {
    pub name: String,
    /// "ok" | "degraded" | "unavailable"
    pub status: String,
    pub ok_count: usize,
    pub failed_count: usize,
    pub detail: String,
}

/// One archived run, reduced to what trend computation needs. This is the
/// record written to `data/history/runs.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrendPoint {
    /// Calendar date of the run (YYYY-MM-DD). At most one point per date is
    /// used, so intraday re-runs cannot masquerade as a longer history.
    pub date: String,
    /// Full timestamp of the run that produced this point.
    pub generated_at: String,
    pub composite: f64,
    pub coverage: f64,
    pub phase: String,
    /// The config schema_version this run was computed under.
    ///
    /// Redefining an indicator changes what the composite MEANS, so runs computed
    /// under different methodology are not comparable even at identical coverage.
    /// Defaulted for archives written before this field existed, which are
    /// treated as "1.0" by `history::methodology_of`.
    #[serde(default)]
    pub methodology_version: String,
    /// Per-indicator stress, keyed by indicator id. Empty entries are absent,
    /// never zero-filled — a missing indicator is not a stress of zero.
    #[serde(default)]
    pub stresses: BTreeMap<String, f64>,
}

/// Direction of a measured change, with a dead-band so noise is not read as a
/// signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Rising,
    Falling,
    Flat,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::Rising => "rising",
            Direction::Falling => "falling",
            Direction::Flat => "flat",
        }
    }
    /// Classify a change against the configured dead-band.
    pub fn classify(delta: f64, flat_band: f64) -> Direction {
        if delta.abs() < flat_band {
            Direction::Flat
        } else if delta > 0.0 {
            Direction::Rising
        } else {
            Direction::Falling
        }
    }
}

/// Change in one indicator since the baseline. Present only when BOTH runs
/// scored that indicator — comparing against a gap would be comparing a number
/// with a non-number.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndicatorTrend {
    pub id: String,
    pub label: String,
    pub current_stress: f64,
    pub baseline_stress: f64,
    pub delta: f64,
    pub direction: Direction,
}

/// A composite-level change against an eligible baseline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrendDelta {
    pub baseline_date: String,
    pub baseline_composite: f64,
    pub baseline_coverage: f64,
    pub composite_delta: f64,
    /// Elapsed days. Always present: a +4 change over 3 days and a +4 change
    /// over 400 days are different facts, so one is never reported without it.
    pub elapsed_days: f64,
    pub direction: Direction,
    pub phase_now: String,
    pub phase_then: String,
    /// True when the phase label changed, which is the event worth noticing.
    pub phase_changed: bool,
    pub indicators: Vec<IndicatorTrend>,
}

/// Direction of travel, or an explicit statement of why it could not be
/// computed. `delta: None` with a non-empty `reason` is the honest degraded
/// state — never a silently absent field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trend {
    /// Recorded daily points, oldest first, newest last.
    pub points: Vec<TrendPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<TrendDelta>,
    /// Why no delta is available, when that is the case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Non-fatal problems found while reading the archive (e.g. a malformed
    /// line). Reported rather than swallowed.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// True when the current run was appended to the archive.
    pub recorded: bool,
}

impl Trend {
    /// No history at all — used when history is disabled or unavailable.
    pub fn empty(reason: &str) -> Trend {
        Trend {
            points: Vec::new(),
            delta: None,
            reason: Some(reason.to_string()),
            warnings: Vec::new(),
            recorded: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Report {
    pub tool: String,
    pub version: String,
    pub generated_at: String,
    pub composite: f64,
    pub coverage: f64,
    pub confidence: String,
    pub phase: String,
    pub phase_label: String,
    pub phase_detail: String,
    /// Plain-English summary for a non-specialist. Generated from the same
    /// numbers as everything else, so it cannot drift out of sync.
    pub layman: crate::report::layman::LaymanSummary,
    pub analog: AnalogWindow,
    /// Direction of travel since an eligible baseline. CONTEXT ONLY — it never
    /// enters the composite.
    pub trend: Trend,
    /// Per-company exposure ranking. CONTEXT ONLY — company-level analysis never
    /// enters a market-level composite.
    #[serde(default)]
    pub exposure: Vec<crate::exposure::CompanyExposure>,
    /// Falsification tests: measurements that could show the bubble thesis is
    /// WRONG. Deliberately NOT in the composite — averaging "evidence for" and
    /// "evidence against" into one number would be a category error.
    #[serde(default)]
    pub falsifiers: Vec<crate::falsifiers::Falsifier>,
    /// GSADF explosiveness test on the price series. A different KIND of evidence
    /// from every indicator here: a formal hypothesis test rather than a
    /// hand-anchored judgement. Deliberately NOT folded into the composite, and
    /// reported even when it disagrees with the composite, because the
    /// disagreement is the informative part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explosiveness: Option<crate::gsadf::GsadfResult>,
    pub indicators: Vec<IndicatorReading>,
    pub data_quality: DataQuality,
    pub sources: Vec<SourceHealth>,
    pub headline: String,
    pub caveats: Vec<String>,
    pub disclaimer: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(end: &str, val: f64) -> EdgarFact {
        EdgarFact {
            tag: "LongTermDebt".into(),
            start: String::new(),
            end: end.into(),
            val,
            form: "10-K".into(),
            filed: end.into(),
            days: 0,
        }
    }

    #[test]
    fn point_in_time_rejects_an_exactly_zero_balance() {
        // No large filer has zero long-term debt; a zero means the tag is wrong.
        // Reporting it would make the most leveraged company look debt-free —
        // which is exactly the ORCL bug this guard exists to prevent.
        let facts = vec![fact("2026-06-30", 0.0)];
        let r = CompanyFacts::point_in_time(&facts, "2026-09-17", 200);
        assert!(r.is_err(), "an exactly-zero balance must be rejected");
        assert!(r.unwrap_err().contains("exactly zero"));
    }

    #[test]
    fn point_in_time_rejects_a_stale_series() {
        // The ORCL bug in miniature: a resolver taking the latest fact regardless
        // of age reports a years-old balance sheet as current.
        let facts = vec![fact("2022-05-31", 0.0), fact("2023-01-01", 5_000_000_000.0)];
        let r = CompanyFacts::point_in_time(&facts, "2026-09-17", 200);
        assert!(r.is_err(), "a stale series must be rejected");
        assert!(r.unwrap_err().contains("stale"));
    }

    #[test]
    fn point_in_time_accepts_a_current_non_zero_balance() {
        let facts = vec![fact("2026-05-31", 122_340_000_000.0)];
        let r = CompanyFacts::point_in_time(&facts, "2026-09-17", 200);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().val, 122_340_000_000.0);
    }

    #[test]
    fn point_in_time_rejects_an_empty_series_with_a_stated_reason() {
        let empty: Vec<EdgarFact> = vec![];
        let e = CompanyFacts::point_in_time(&empty, "2026-09-17", 200).unwrap_err();
        assert!(e.contains("no facts"), "must say why, not just fail: {}", e);
    }

    #[test]
    fn point_in_time_picks_the_newest_fact_not_the_first() {
        // Ordering must not matter: EDGAR does not guarantee it.
        let facts = vec![fact("2026-05-31", 10.0), fact("2025-05-31", 999.0)];
        let r = CompanyFacts::point_in_time(&facts, "2026-09-17", 400).unwrap();
        assert_eq!(r.val, 10.0, "must take the newest, not the first listed");
    }
}
