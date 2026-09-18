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

    // Series length and span, so the reader can see whether the scale is anchored
    // on observed history or on extrapolation. This is the concrete reason the
    // investment-grade indicator moved to a 40-year series.
    let n = s.points.len();
    let span = match (s.points.first(), s.points.last()) {
        (Some(a), Some(b)) => format!("{} to {}", a.date, b.date),
        _ => "unknown span".to_string(),
    };
    let history_note = if n < 200 {
        format!(
            " HISTORY WARNING: this series has only {} observations ({}), which spans too \
             little time to contain a full stress episode, so the upper part of the scale is \
             extrapolated rather than observed.",
            n, span
        )
    } else {
        format!(" History: {} observations spanning {}.", n, span)
    };

    // If another part of the report reads the same series, say so. Counting one
    // series twice as though it were two pieces of evidence is the exact
    // double-counting error already fixed once in this model.
    let shared = if id == "credit_ig" && SHARES_SERIES_WITH_FALSIFIER {
        format!(
            " NOT INDEPENDENT: the falsification test for credit conditions reads the same {} \
             series, so this indicator and that test are the same measurement read two ways \
             (a level here, a historical percentile there). Do not treat them as two pieces \
             of evidence.",
            series_id
        )
    } else {
        String::new()
    };

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
             is the early warning. Watch the change, not the level.{}{}",
            what, level, s.provenance.as_of, history_note, shared
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

/// Investment-grade credit conditions, measured on a series with 40 years of
/// history rather than two and a half.
///
/// WHY THE SERIES CHANGED. This indicator previously read the ICE BofA US Corporate
/// option-adjusted spread, which is the *better* measure of investment-grade
/// conditions — but every ICE BofA series on FRED begins 2023-09-18. Verified from
/// this host 2026-09-17: BAMLH0A0HYM2, BAMLC0A0CM, BAMLH0A1HYBB and the rest all
/// start on that date, and no free pre-2023 high-yield or IG option-adjusted spread
/// exists.
///
/// The consequence was not cosmetic. With only ~2.5 years of history the anchors
/// could not be anchored on any observed stress episode: the series had never seen
/// 2000, 2008 or 2020, so the top of its scale was an extrapolation rather than a
/// reference. Baa-minus-10Y carries 10,177 observations from 1986 and includes all
/// three episodes, so the same scale can now be justified by history.
///
/// WHAT IT MEASURES NOW: the yield on Baa-rated corporate debt minus the 10-year
/// Treasury — a credit-risk premium for the lower half of investment grade.
///
/// DIRECTION is unchanged: wide = high stress, tight = low stress. The config
/// already documents why the complacency reading (tight spreads as the bubble
/// signal) is deliberately not used: HY OAS has a ~3% median over its record and is
/// rarely wide, so a level-based complacency score would read most of the last
/// decade as a bubble — near-constant, and carrying no timing information.
///
/// ONE HONEST CONSEQUENCE, disclosed rather than left for a reader to notice: the
/// falsification test for credit conditions also uses BAA10Y, so this indicator and
/// that test are NOT independent measurements. They are the same series read two
/// ways — a level-scored indicator and a historical-percentile falsifier. The output
/// says so, because counting one series twice as though it were two pieces of
/// evidence is exactly the double-counting error already fixed once in this model.
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
            "Moody's Baa corporate yield less the 10-year Treasury",
        )
    }
}

/// True when the same series backs another part of the report, so the output can
/// say that the two are not independent. Kept as a named function rather than a
/// comment so the statement lives in the code that renders it.
pub const SHARES_SERIES_WITH_FALSIFIER: bool = true;

/// Private-credit lending growth: the financing channel the public spreads miss.
///
/// WHY THIS INDICATOR EXISTS. `credit_hy` and `credit_ig` read PUBLIC bond index
/// spreads. BIS Bulletin 120 (Jan 2026) documents that AI-related financing is
/// shifting from operating cash flow toward debt with **private credit playing a
/// rapidly increasing role** — and private credit appears in no index spread,
/// because it is not traded. This measures that channel's size and growth.
///
/// Measured from this host 2026-09-17, from the Fed's own F4.4 table:
///   all sectors' private-credit loans   $1.1tn (2024:Q1) -> $1.5tn (2026:Q2),
///   roughly +36% in ten quarters, while the public high-yield index spread has
///   barely moved.
///
/// THE LIMITATION IS SEVERE AND IS STATED IN THE OUTPUT, not only here. Z.1
/// reports private credit as a TOTAL for the entire economy. It is NOT AI-specific
/// and nothing in the free data attributes a private loan to a data-centre or GPU
/// borrower. So this measures the size and growth of the CHANNEL, not the amount
/// reaching AI. A high reading is consistent with the BIS thesis and does not
/// confirm it. It is included because a growing share of this cycle's debt is
/// being originated where the tool was not looking, and silently ignoring that
/// would be worse than measuring it with a caveat.
///
/// SCORED ON GROWTH, NOT LEVEL. The level of economy-wide private credit is not a
/// bubble signal at all; a large and stable stock is unremarkable. What matters is
/// rapid expansion, which is what "private credit is playing a rapidly increasing
/// role" actually asserts. Scoring the level would read every year as alarming.
pub struct PrivateCreditGrowth;

impl Indicator for PrivateCreditGrowth {
    fn id(&self) -> &'static str {
        "private_credit_growth"
    }
    fn evaluate(&self, ctx: &Ctx) -> Reading {
        let ic = match ctx.cfg.indicator(self.id()) {
            Some(c) => c,
            None => {
                return Reading::Unavailable {
                    reason: "not configured".into(),
                }
            }
        };

        let Some(series) = ctx.obs.yahoo.get("PRIVATE_CREDIT_ALL") else {
            return Reading::Unavailable {
                reason: "Fed Z.1 private-credit series unavailable: the release archive could \
                         not be retrieved or the table was not parseable"
                    .into(),
            };
        };

        let Some(growth) = crate::sources::z1::yoy_growth(&series.points) else {
            return Reading::Unavailable {
                reason: format!(
                    "private-credit series has only {} observations, too few for a \
                     year-over-year comparison",
                    series.points.len()
                ),
            };
        };

        let last = series.points.last();
        let prior = series.points.get(series.points.len().saturating_sub(5));
        let level_note = match (last, prior) {
            (Some(l), Some(p)) => format!(
                "Level: ${:.2}tn at {}, against ${:.2}tn a year earlier.",
                l.value / 1e12,
                l.date,
                p.value / 1e12
            ),
            _ => String::new(),
        };

        let stress = crate::score::interpolate(growth, &ic.anchors);

        Reading::Scored {
            stress,
            value: growth,
            unit: ic.unit.clone(),
            detail: format!(
                "Private-credit lending grew {:.1}% year over year. {}. This is the channel \
                 BIS Bulletin 120 identifies as taking a rapidly increasing role in financing \
                 the buildout, and it appears in NEITHER of the public credit-spread indicators \
                 above because private credit is not traded and so has no index spread. \
                 CRITICAL LIMITATION, please do not over-read this number: Z.1 reports private \
                 credit as a TOTAL FOR THE WHOLE ECONOMY. It is not AI-specific, and nothing in \
                 the free data attributes an individual private loan to a data-centre or GPU \
                 borrower. This measures the size and growth of the financing CHANNEL, not the \
                 amount of it reaching AI. Growth in the channel is consistent with the BIS \
                 thesis and does not establish it. Scored on growth rather than level: the \
                 outstanding stock of private credit is not itself a bubble signal, whereas \
                 rapid expansion is what the BIS claim actually asserts.",
                growth, level_note
            ),
            provenance: series.provenance.clone(),
        }
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
            gsadf: GsadfCfg::default(),
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
