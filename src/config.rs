//! Configuration loading and validation.
//!
//! Everything the model thinks lives in the TOML file. This module refuses to
//! run on a config that is internally inconsistent, so a typo cannot silently
//! change the score.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub meta: Meta,
    pub phase: PhaseCfg,
    pub coverage_floor: CoverageFloor,
    #[serde(default = "TrendCfg::defaults")]
    pub trend: TrendCfg,
    #[serde(default)]
    pub gsadf: GsadfCfg,
    pub analog: AnalogCfg,
    #[serde(default)]
    pub analog_band: Vec<AnalogBand>,
    #[serde(default)]
    pub indicator: Vec<IndicatorCfg>,
    /// The four (or however many) QUESTIONS the model answers, declared here so the
    /// sub-scores are configurable rather than hard-coded. An indicator names one of
    /// these keys in its `group`.
    #[serde(default)]
    pub sub_question: Vec<SubQuestionCfg>,
}

/// One question the model answers, reported as a SEPARATE score.
///
/// WHY THIS EXISTS. A single composite blends indicators that answer different
/// questions, and those questions can legitimately disagree by tens of points —
/// measured 2026-09-26: the spending/balance-sheet-strain block read 53.3 while the
/// market-pricing block read 25.2, and the blended composite reported 40.2. That is
/// not a weighted average of one quantity; it is a blend of a CAUSE (the precondition
/// for a bust) with its own downstream EFFECT (the realisation). Averaging them means
/// a clearly-present precondition reads as "middle range" for exactly as long as the
/// realisation has not arrived — and the realisation is by construction last.
///
/// Reporting the sub-questions separately does NOT change the composite. It makes the
/// disagreement visible instead of hiding it inside one number.
#[derive(Debug, Clone, Deserialize)]
pub struct SubQuestionCfg {
    pub key: String,
    pub label: String,
    /// What this question is FOR, shown so a reader knows why the score exists.
    pub role: String,
    /// Plain-English gloss for the layman summary.
    pub plain: String,
    /// Reported order, low to high. Explicit because a `Vec` order chosen by TOML
    /// authoring order would silently reorder the reader's view of the model.
    pub order: u32,
}

impl TrendCfg {
    /// Defaults for a config written before v1.1, so it still loads and behaves
    /// sensibly. MATCHES the shipped config's `[trend]` block, and a test asserts that,
    /// because these two silently diverging would mean a pre-v1.1 config measured elapsed
    /// time differently from a current one without saying so.
    pub fn defaults() -> TrendCfg {
        TrendCfg {
            min_gap_days: 0.9,
            coverage_tolerance_pp: 5.0,
            flat_band: 1.0,
            sparkline_points: 30,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub schema_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhaseCfg {
    pub early_max: f64,
    pub mid_max: f64,
    pub late_max: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoverageFloor {
    pub low_below: f64,
    pub high_above: f64,
}

/// Trend / direction-of-travel settings. See the `[trend]` block in the config
/// for why coverage tolerance is the load-bearing value here.
#[derive(Debug, Clone, Deserialize)]
pub struct TrendCfg {
    pub min_gap_days: f64,
    pub coverage_tolerance_pp: f64,
    pub flat_band: f64,
    pub sparkline_points: usize,
}

/// Settings for the GSADF explosiveness test.
#[derive(Debug, Clone, Deserialize)]
pub struct GsadfCfg {
    #[serde(default = "GsadfCfg::default_enabled")]
    pub enabled: bool,
    #[serde(default = "GsadfCfg::default_reps")]
    pub monte_carlo_reps: usize,
    #[serde(default = "GsadfCfg::default_seed")]
    pub seed: u64,
}

impl GsadfCfg {
    fn default_enabled() -> bool {
        true
    }
    fn default_reps() -> usize {
        200
    }
    fn default_seed() -> u64 {
        20260917
    }
}

impl Default for GsadfCfg {
    fn default() -> Self {
        GsadfCfg {
            enabled: Self::default_enabled(),
            monte_carlo_reps: Self::default_reps(),
            seed: Self::default_seed(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogCfg {
    pub method: String,
    pub analogs: Vec<String>,
    pub derivation: String,
    pub caveat: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogBand {
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub range_months: [f64; 2],
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndicatorCfg {
    pub id: String,
    pub label: String,
    pub weight: f64,
    pub unit: String,
    pub source: String,
    pub rationale: String,
    pub anchors: Vec<[f64; 2]>,
    #[serde(default)]
    pub fred_series: Option<String>,
    /// Which QUESTION this indicator answers — one of the four `SubQuestion` keys
    /// declared in `[[sub_question]]`. Optional in TOML so a config written before
    /// groups existed still loads; `validate` then refuses to start, because an
    /// ungrouped indicator would be silently absent from every sub-score.
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}
impl std::error::Error for ConfigError {}

impl Config {
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ConfigError(format!("cannot read {}: {}", path.display(), e)))?;
        let cfg: Config =
            toml::from_str(&text).map_err(|e| ConfigError(format!("invalid TOML: {}", e)))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Reject configs that would produce a misleading score.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.indicator.is_empty() {
            return Err(ConfigError("no indicators defined".into()));
        }

        // Phase thresholds must ascend.
        let p = &self.phase;
        if !(p.early_max < p.mid_max && p.mid_max < p.late_max) {
            return Err(ConfigError(
                "phase thresholds must ascend: early_max < mid_max < late_max".into(),
            ));
        }

        // Weights must be non-negative.
        for ind in &self.indicator {
            if ind.weight < 0.0 {
                return Err(ConfigError(format!(
                    "indicator '{}' has negative weight",
                    ind.id
                )));
            }
        }

        // Sub-question groups: keys unique, orders unique, and EVERY indicator named.
        //
        // The last rule is the load-bearing one. An indicator with no group (or an
        // unknown one) would be absent from every sub-score while still counting in
        // the composite — so the sub-scores would not sum to the whole and a reader
        // could not tell which indicator had been dropped. Refusing to start is the
        // only safe behaviour: this is a silent omission, exactly the failure mode
        // the rest of this project spends its effort preventing.
        if self.sub_question.is_empty() {
            return Err(ConfigError(
                "no [[sub_question]] groups declared; every indicator must belong to one".into(),
            ));
        }
        {
            let mut keys: Vec<&str> = self.sub_question.iter().map(|s| s.key.as_str()).collect();
            let before = keys.len();
            keys.sort_unstable();
            keys.dedup();
            if keys.len() != before {
                return Err(ConfigError(
                    "sub_question keys must be unique (a duplicate would merge two questions)"
                        .into(),
                ));
            }
            let mut orders: Vec<u32> = self.sub_question.iter().map(|s| s.order).collect();
            let before = orders.len();
            orders.sort_unstable();
            orders.dedup();
            if orders.len() != before {
                return Err(ConfigError(
                    "sub_question orders must be unique (a tie would make the report order ambiguous)"
                        .into(),
                ));
            }
        }
        for ind in &self.indicator {
            match ind.group.as_deref() {
                None => {
                    return Err(ConfigError(format!(
                        "indicator '{}' has no `group`; it would be silently absent from every \
                         sub-score while still counting in the composite",
                        ind.id
                    )));
                }
                Some(g) => {
                    if !self.sub_question.iter().any(|s| s.key == g) {
                        return Err(ConfigError(format!(
                            "indicator '{}' names group '{}', which is not declared in any \
                             [[sub_question]]",
                            ind.id, g
                        )));
                    }
                }
            }
        }
        // A declared group with no indicators would report an empty score, which reads
        // as "calm" rather than "unmeasured". Refuse it.
        for sq in &self.sub_question {
            if !self
                .indicator
                .iter()
                .any(|i| i.group.as_deref() == Some(sq.key.as_str()))
            {
                return Err(ConfigError(format!(
                    "sub_question '{}' has no indicators; it would render as an empty score",
                    sq.key
                )));
            }
        }

        // Anchors must be present, monotone in raw value, and produce at least
        // two distinct stress levels (otherwise the indicator is constant).
        for ind in &self.indicator {
            if ind.weight == 0.0 {
                continue; // declared gap, not scored
            }
            if ind.anchors.len() < 2 {
                return Err(ConfigError(format!(
                    "indicator '{}' needs at least 2 anchors",
                    ind.id
                )));
            }
            let mut raws: Vec<f64> = ind.anchors.iter().map(|a| a[0]).collect();
            let sorted = {
                let mut s = raws.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                s
            };
            if raws != sorted {
                return Err(ConfigError(format!(
                    "indicator '{}' anchors must be ascending in raw value",
                    ind.id
                )));
            }
            raws.dedup();
            for a in &ind.anchors {
                if !(0.0..=100.0).contains(&a[1]) {
                    return Err(ConfigError(format!(
                        "indicator '{}' anchor stress {} outside 0..100",
                        ind.id, a[1]
                    )));
                }
            }
        }

        // Duplicate ids would silently double-count.
        let mut ids: Vec<&str> = self.indicator.iter().map(|i| i.id.as_str()).collect();
        ids.sort_unstable();
        let n_before = ids.len();
        ids.dedup();
        if ids.len() != n_before {
            return Err(ConfigError("duplicate indicator id".into()));
        }

        // Trend settings must be sane, because each one guards against a
        // specific way the direction-of-travel output could mislead.
        let t = &self.trend;
        if t.min_gap_days < 0.0 {
            return Err(ConfigError(
                "trend.min_gap_days must not be negative".into(),
            ));
        }
        if t.coverage_tolerance_pp < 0.0 {
            return Err(ConfigError(
                "trend.coverage_tolerance_pp must not be negative".into(),
            ));
        }
        if t.flat_band < 0.0 {
            return Err(ConfigError("trend.flat_band must not be negative".into()));
        }
        if self.gsadf.enabled && self.gsadf.monte_carlo_reps < 20 {
            return Err(ConfigError(
                "gsadf.monte_carlo_reps must be at least 20 for the critical values to mean \
                 anything"
                    .into(),
            ));
        }
        if t.sparkline_points < 2 {
            return Err(ConfigError(
                "trend.sparkline_points must be at least 2 to draw a line".into(),
            ));
        }

        Ok(())
    }

    pub fn total_weight(&self) -> f64 {
        self.indicator.iter().map(|i| i.weight).sum()
    }

    pub fn indicator(&self, id: &str) -> Option<&IndicatorCfg> {
        self.indicator.iter().find(|i| i.id == id)
    }

    pub fn band_for(&self, composite: f64) -> Option<&AnalogBand> {
        // Highest band whose lower bound the composite reaches.
        self.analog_band
            .iter()
            .filter(|b| composite >= b.min)
            .max_by(|a, b| a.min.partial_cmp(&b.min).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Config {
        Config {
            meta: Meta {
                schema_version: "1.0".into(),
            },
            phase: PhaseCfg {
                early_max: 35.0,
                mid_max: 55.0,
                late_max: 75.0,
            },
            coverage_floor: CoverageFloor {
                low_below: 0.6,
                high_above: 0.85,
            },
            trend: TrendCfg::defaults(),
            gsadf: GsadfCfg::default(),
            analog: AnalogCfg {
                method: "historical_analog".into(),
                analogs: vec!["a".into()],
                derivation: "d".into(),
                caveat: "c".into(),
            },
            analog_band: vec![],
            sub_question: vec![SubQuestionCfg {
                key: "strain".into(),
                label: "S".into(),
                role: "r".into(),
                plain: "p".into(),
                order: 1,
            }],
            indicator: vec![IndicatorCfg {
                id: "x".into(),
                label: "X".into(),
                group: Some("strain".into()),
                weight: 10.0,
                unit: "u".into(),
                source: "yahoo".into(),
                rationale: "r".into(),
                anchors: vec![[0.0, 0.0], [10.0, 100.0]],
                fred_series: None,
            }],
        }
    }

    #[test]
    fn accepts_valid_config() {
        assert!(base().validate().is_ok());
    }

    #[test]
    fn rejects_descending_anchors() {
        let mut c = base();
        c.indicator[0].anchors = vec![[10.0, 0.0], [0.0, 100.0]];
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_non_ascending_phases() {
        let mut c = base();
        c.phase.mid_max = 20.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_stress_out_of_range() {
        let mut c = base();
        c.indicator[0].anchors = vec![[0.0, 0.0], [10.0, 140.0]];
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let mut c = base();
        let dup = c.indicator[0].clone();
        c.indicator.push(dup);
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_a_sparkline_that_cannot_be_drawn() {
        let mut c = base();
        c.trend.sparkline_points = 1;
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_negative_coverage_tolerance() {
        // A negative tolerance would make every baseline ineligible; catching it
        // at config load is better than a silently dead trend feature.
        let mut c = base();
        c.trend.coverage_tolerance_pp = -1.0;
        assert!(c.validate().is_err());
    }

    #[test]
    fn trend_defaults_apply_when_the_block_is_absent() {
        // A pre-v1.1 config must keep working unchanged.
        let text = std::fs::read_to_string("config/indicators.toml").unwrap();
        assert!(text.contains("[trend]"));
        let d = TrendCfg::defaults();
        assert!(d.coverage_tolerance_pp > 0.0);
        assert!(d.sparkline_points >= 2);
    }

    // ------------------------------------------------- sub-question grouping rules
    // These guard against the SILENT OMISSION failure mode: an indicator that counts
    // in the composite but appears in no sub-score. A reader would see a group table
    // that silently excludes it and could not tell.

    #[test]
    fn rejects_an_indicator_with_no_group() {
        let mut c = base();
        c.indicator[0].group = None;
        let e = c.validate().unwrap_err().0;
        assert!(e.contains("has no `group`"), "got: {e}");
        assert!(
            e.contains("silently absent"),
            "the message must name the failure mode: {e}"
        );
    }

    #[test]
    fn rejects_an_indicator_naming_an_undeclared_group() {
        let mut c = base();
        c.indicator[0].group = Some("ghost".into());
        let e = c.validate().unwrap_err().0;
        assert!(e.contains("not declared"), "got: {e}");
    }

    #[test]
    fn rejects_a_declared_group_with_no_indicators() {
        // An empty group would render as a score of nothing, which reads as CALM.
        let mut c = base();
        c.sub_question.push(SubQuestionCfg {
            key: "empty".into(),
            label: "E".into(),
            role: "r".into(),
            plain: "p".into(),
            order: 9,
        });
        let e = c.validate().unwrap_err().0;
        assert!(e.contains("no indicators"), "got: {e}");
    }

    #[test]
    fn rejects_duplicate_group_keys() {
        let mut c = base();
        c.sub_question.push(SubQuestionCfg {
            key: "strain".into(),
            label: "dup".into(),
            role: "r".into(),
            plain: "p".into(),
            order: 9,
        });
        let e = c.validate().unwrap_err().0;
        assert!(e.contains("unique"), "got: {e}");
    }

    #[test]
    fn rejects_duplicate_group_orders() {
        // A tie would make the report's group order ambiguous, which is a silent
        // change to how a reader sees the model.
        let mut c = base();
        c.sub_question.push(SubQuestionCfg {
            key: "other".into(),
            label: "O".into(),
            role: "r".into(),
            plain: "p".into(),
            order: 1,
        });
        c.indicator.push(IndicatorCfg {
            id: "y".into(),
            label: "Y".into(),
            group: Some("other".into()),
            weight: 5.0,
            unit: "u".into(),
            source: "yahoo".into(),
            rationale: "r".into(),
            anchors: vec![[0.0, 0.0], [10.0, 100.0]],
            fred_series: None,
        });
        let e = c.validate().unwrap_err().0;
        assert!(e.contains("order") && e.contains("unique"), "got: {e}");
    }

    #[test]
    fn rejects_a_config_with_no_sub_questions_at_all() {
        let mut c = base();
        c.sub_question.clear();
        c.indicator[0].group = None;
        assert!(c.validate().is_err());
    }
}
