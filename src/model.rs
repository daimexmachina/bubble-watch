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

    /// Most recent fact of any duration, for point-in-time series like shares.
    pub fn latest(facts: &[EdgarFact]) -> Option<&EdgarFact> {
        facts.iter().max_by(|a, b| a.end.cmp(&b.end))
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
    pub indicators: Vec<IndicatorReading>,
    pub data_quality: DataQuality,
    pub sources: Vec<SourceHealth>,
    pub headline: String,
    pub caveats: Vec<String>,
    pub disclaimer: String,
}
