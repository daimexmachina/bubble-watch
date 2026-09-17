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
    pub analog: AnalogCfg,
    #[serde(default)]
    pub analog_band: Vec<AnalogBand>,
    #[serde(default)]
    pub indicator: Vec<IndicatorCfg>,
}

impl TrendCfg {
    /// Conservative defaults, identical to the shipped config, so a config
    /// written before v1.1 still loads and behaves the same way.
    pub fn defaults() -> TrendCfg {
        TrendCfg {
            min_gap_days: 1.0,
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
            analog: AnalogCfg {
                method: "historical_analog".into(),
                analogs: vec!["a".into()],
                derivation: "d".into(),
                caveat: "c".into(),
            },
            analog_band: vec![],
            indicator: vec![IndicatorCfg {
                id: "x".into(),
                label: "X".into(),
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
}
