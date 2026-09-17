//! Credit indicators — the non-equity half of the picture.
//!
//! These depend on FRED, which is an OPTIONAL source: during probing it stopped
//! answering this host entirely. When it is down these two indicators are
//! reported as gaps and the composite rests on less. That is the correct
//! behaviour — but it means the composite becomes an *equity-price-heavy*
//! reading, and the report says so explicitly in its caveats.

use super::{Ctx, Indicator};
use crate::model::Reading;

fn credit_reading(ctx: &Ctx, id: &str, _label: &str, what: &str) -> Reading {
    let ic = match ctx.cfg.indicator(id) {
        Some(c) => c,
        None => {
            return Reading::Unavailable {
                reason: "not configured".into(),
            }
        }
    };
    let Some(series_id) = ic.fred_series.as_deref() else {
        return Reading::Unavailable {
            reason: format!("indicator '{}' has no fred_series configured", id),
        };
    };
    let Some(s) = ctx.obs.fred.get(series_id) else {
        let why = ctx
            .obs
            .failures
            .iter()
            .find(|f| f.source == "fred" && f.endpoint == series_id)
            .map(|f| f.reason.clone())
            .unwrap_or_else(|| "not retrieved".into());
        return Reading::Unavailable {
            reason: format!(
                "FRED series {} unavailable ({}) — OPTIONAL source, treated as a coverage gap",
                series_id, why
            ),
        };
    };
    let Some(level) = s.latest_value() else {
        return Reading::Unavailable {
            reason: format!("FRED series {} returned no observations", series_id),
        };
    };
    let stress = crate::score::interpolate(level, &ic.anchors);
    Reading::Scored {
        stress,
        value: level,
        unit: ic.unit.clone(),
        detail: format!(
            "{} {} as of {}. Direction: WIDE spread = high stress, tight = low stress \
             (not inverted). A tight spread means credit is currently cheap and the market \
             is charging little for this risk; it is NOT scored as a bubble signal, because \
             HY spreads have a ~3.0% median over the available record and a level-based \
             complacency score would read nearly the whole post-2009 period as a bubble. \
             The informative signal is the DIRECTION OF TRAVEL from a tight base — widening \
             is the early warning. Watch the change, not the level.",
            what, level, s.provenance.as_of
        ),
        provenance: s.provenance.clone(),
    }
}

pub struct CreditHy;
impl Indicator for CreditHy {
    fn id(&self) -> &'static str {
        "credit_hy"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        credit_reading(
            ctx,
            self.id(),
            "US high-yield credit spreads",
            "ICE BofA US High Yield option-adjusted spread",
        )
    }
}

pub struct CreditIg;
impl Indicator for CreditIg {
    fn id(&self) -> &'static str {
        "credit_ig"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        credit_reading(
            ctx,
            self.id(),
            "US investment-grade credit spreads",
            "ICE BofA US Corporate option-adjusted spread",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::model::{Observations, Point, Provenance, Series};

    /// A minimal config carrying just the two credit indicators, so these tests
    /// do not depend on the shipped file or on the current working directory.
    fn cfg() -> Config {
        Config {
            meta: Meta {
                schema_version: "1".into(),
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
                analogs: vec![],
                derivation: String::new(),
                caveat: String::new(),
            },
            analog_band: vec![],
            indicator: vec![
                IndicatorCfg {
                    id: "credit_hy".into(),
                    label: "HY".into(),
                    weight: 12.0,
                    unit: "pct".into(),
                    source: "fred".into(),
                    rationale: String::new(),
                    // Same shape as the shipped anchors: tight spread = LOW stress.
                    anchors: vec![
                        [2.0, 5.0],
                        [2.8, 18.0],
                        [3.5, 30.0],
                        [5.0, 50.0],
                        [7.0, 75.0],
                        [10.0, 95.0],
                    ],
                    fred_series: Some("BAMLH0A0HYM2".into()),
                },
                IndicatorCfg {
                    id: "credit_ig".into(),
                    label: "IG".into(),
                    weight: 6.0,
                    unit: "pct".into(),
                    source: "fred".into(),
                    rationale: String::new(),
                    anchors: vec![[0.5, 8.0], [0.8, 20.0], [1.5, 55.0]],
                    fred_series: Some("BAMLC0A0CM".into()),
                },
            ],
        }
    }

    /// Synthetic series built for the test. These are deliberately constructed
    /// INPUTS for exercising arithmetic — they are not presented as observations
    /// of the real market anywhere in the tool's output.
    fn synth(vals: &[f64]) -> Series {
        Series {
            provenance: Provenance {
                source: "synthetic-test-input".into(),
                endpoint: "test://synthetic".into(),
                as_of: "2026-09-16".into(),
                retrieved_at: "2026-09-16T00:00:00Z".into(),
            },
            points: vals
                .iter()
                .enumerate()
                .map(|(i, v)| Point {
                    date: format!("2026-09-{:02}", (i % 28) + 1),
                    value: *v,
                })
                .collect(),
        }
    }

    fn ctx_with<'a>(cfg: &'a Config, obs: &'a Observations) -> Ctx<'a> {
        Ctx { obs, cfg }
    }

    #[test]
    fn tight_spread_scores_low_stress() {
        // Not inverted: a tight spread is cheap credit, reported as low stress.
        // The informative signal is direction of travel, not level.
        let c = cfg();
        let mut obs = Observations::default();
        obs.fred
            .insert("BAMLH0A0HYM2".into(), synth(&[2.4, 2.6, 2.76]));
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Scored { stress, value, .. } => {
                assert!((value - 2.76).abs() < 1e-9);
                assert!(
                    stress < 25.0,
                    "tight spread should read calm, got {}",
                    stress
                );
            }
            other => panic!("expected a scored reading, got {:?}", other),
        }
    }

    #[test]
    fn wide_spread_scores_high_stress() {
        let c = cfg();
        let mut obs = Observations::default();
        obs.fred
            .insert("BAMLH0A0HYM2".into(), synth(&[2.76, 5.5, 8.0]));
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Scored { stress, value, .. } => {
                assert!((value - 8.0).abs() < 1e-9);
                assert!(
                    stress > 70.0,
                    "wide spread should read stressed, got {}",
                    stress
                );
            }
            other => panic!("expected a scored reading, got {:?}", other),
        }
    }

    #[test]
    fn stress_increases_monotonically_as_spreads_widen() {
        let c = cfg();
        let mut prev = -1.0;
        for level in [2.0, 3.0, 4.0, 5.0, 7.0, 10.0] {
            let mut obs = Observations::default();
            obs.fred.insert("BAMLH0A0HYM2".into(), synth(&[level]));
            if let Reading::Scored { stress, .. } = CreditHy.evaluate(&ctx_with(&c, &obs)) {
                assert!(
                    stress > prev,
                    "stress must rise with spread: {} vs {}",
                    stress,
                    prev
                );
                prev = stress;
            } else {
                panic!("expected scored at {}", level);
            }
        }
    }

    #[test]
    fn missing_fred_series_is_a_reported_gap_not_a_zero() {
        let c = cfg();
        let obs = Observations::default();
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        assert!(!r.is_available(), "must not invent a credit reading");
        match r {
            Reading::Unavailable { reason } => {
                assert!(
                    reason.contains("BAMLH0A0HYM2"),
                    "gap must name the series: {}",
                    reason
                );
                assert!(
                    reason.contains("OPTIONAL"),
                    "gap must state that FRED is optional: {}",
                    reason
                );
            }
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn ig_indicator_uses_its_own_series_and_anchors() {
        let c = cfg();
        let mut obs = Observations::default();
        obs.fred.insert("BAMLC0A0CM".into(), synth(&[1.5]));
        // HY is absent, IG present: IG must still score.
        let r = CreditIg.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Scored { stress, .. } => {
                assert!(
                    (stress - 55.0).abs() < 1e-6,
                    "anchor hit expected, got {}",
                    stress
                );
            }
            other => panic!("expected scored, got {:?}", other),
        }
    }

    #[test]
    fn credit_gap_reason_surfaces_the_upstream_failure() {
        let c = cfg();
        let mut obs = Observations::default();
        obs.failures.push(crate::model::SourceFailure {
            source: "fred".into(),
            endpoint: "BAMLH0A0HYM2".into(),
            reason: "connection reset by peer".into(),
        });
        let r = CreditHy.evaluate(&ctx_with(&c, &obs));
        match r {
            Reading::Unavailable { reason } => assert!(
                reason.contains("connection reset by peer"),
                "the real cause must be propagated, got: {}",
                reason
            ),
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }
}
