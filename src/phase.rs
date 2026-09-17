//! Phase classification and the historical-analog overlay.
//!
//! This module is where the project could most easily become "financial
//! astrology", so it is written defensively: the analog output is explicitly
//! typed as NOT a probability, carries the sample size, and is suppressed
//! entirely below a coverage floor — because a "months to peak" estimate
//! computed from half the inputs would be exactly the fabricated precision this
//! tool exists to avoid.

use crate::config::Config;
use crate::model::AnalogWindow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Early,
    Mid,
    Late,
    Critical,
}

impl Phase {
    pub fn id(&self) -> &'static str {
        match self {
            Phase::Early => "early",
            Phase::Mid => "mid",
            Phase::Late => "late",
            Phase::Critical => "critical",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Early => "Early — little historical bubble evidence in the measured set",
            Phase::Mid => "Mid — clear speculative characteristics, well short of prior extremes",
            Phase::Late => "Late — most measured dimensions near or above prior peak levels",
            Phase::Critical => {
                "Critical — measured configuration at or beyond prior extreme readings"
            }
        }
    }
    pub fn detail(&self) -> &'static str {
        match self {
            Phase::Early => {
                "Most indicators are within ordinary ranges. Note that bubbles are often only \
                 clearly identifiable in hindsight; a calm reading is not evidence that no bubble exists."
            }
            Phase::Mid => {
                "Several indicators show speculative excess while others remain unremarkable. \
                 Capital Economics places volatility and leverage in roughly this category today."
            }
            Phase::Late => {
                "This is the regime the current US AI complex is most often placed in by sell-side \
                 and central-bank analysts. Historically such states have been followed by a peak \
                 within a wide range of horizons — and have also persisted for extended periods."
            }
            Phase::Critical => {
                "Measured dimensions meet or exceed the most extreme readings in the reference \
                 analogs. Extremes are unstable, but an extreme is not a trigger: it carries no \
                 timing information on its own."
            }
        }
    }

    pub fn classify(composite: f64, cfg: &Config) -> Phase {
        let p = &cfg.phase;
        if composite < p.early_max {
            Phase::Early
        } else if composite < p.mid_max {
            Phase::Mid
        } else if composite < p.late_max {
            Phase::Late
        } else {
            Phase::Critical
        }
    }
}

/// The minimum coverage at which the timing overlay is shown at all.
///
/// Rationale: the overlay is the single most misusable output in the tool. If
/// the underlying composite rests on a minority of the intended weight, we
/// report the score but refuse to attach a time range to it.
pub const ANALOG_MIN_COVERAGE: f64 = 0.75;

pub fn analog_window(composite: Option<f64>, coverage: f64, cfg: &Config) -> AnalogWindow {
    let a = &cfg.analog;
    let mut w = AnalogWindow {
        method: a.method.clone(),
        is_probability: false,
        analogs: a.analogs.clone(),
        derivation: a.derivation.clone(),
        caveat: a.caveat.clone(),
        range_months: None,
        band: None,
        band_note: None,
    };

    let Some(c) = composite else { return w };

    if coverage < ANALOG_MIN_COVERAGE {
        w.band_note = Some(format!(
            "Suppressed: weighted coverage {:.0}% is below the {:.0}% floor required to attach a \
             time range. Too much of the intended weight is unavailable for this comparison to \
             mean anything.",
            coverage * 100.0,
            ANALOG_MIN_COVERAGE * 100.0
        ));
        return w;
    }

    if let Some(b) = cfg.band_for(c) {
        w.band = Some(b.label.clone());
        w.range_months = Some(b.range_months);
        w.band_note = Some(b.note.clone());
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn cfg() -> Config {
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
            analog: AnalogCfg {
                method: "historical_analog".into(),
                analogs: vec!["dot-com".into()],
                derivation: "d".into(),
                caveat: "c".into(),
            },
            analog_band: vec![
                AnalogBand {
                    label: "early".into(),
                    min: 0.0,
                    max: 35.0,
                    range_months: [30.0, 72.0],
                    note: "n".into(),
                },
                AnalogBand {
                    label: "late".into(),
                    min: 55.0,
                    max: 75.0,
                    range_months: [6.0, 24.0],
                    note: "n".into(),
                },
            ],
            indicator: vec![],
        }
    }

    #[test]
    fn phase_boundaries_are_exact() {
        let c = cfg();
        assert_eq!(Phase::classify(34.9, &c), Phase::Early);
        assert_eq!(Phase::classify(35.0, &c), Phase::Mid);
        assert_eq!(Phase::classify(54.9, &c), Phase::Mid);
        assert_eq!(Phase::classify(55.0, &c), Phase::Late);
        assert_eq!(Phase::classify(75.0, &c), Phase::Critical);
    }

    #[test]
    fn analog_is_never_a_probability() {
        let w = analog_window(Some(60.0), 1.0, &cfg());
        assert!(!w.is_probability);
        assert_eq!(w.method, "historical_analog");
    }

    #[test]
    fn analog_suppressed_below_coverage_floor() {
        let w = analog_window(Some(60.0), 0.5, &cfg());
        assert!(
            w.range_months.is_none(),
            "must not emit a time range on thin data"
        );
        assert!(w.band_note.as_deref().unwrap().contains("Suppressed"));
    }

    #[test]
    fn analog_reports_range_at_full_coverage() {
        let w = analog_window(Some(60.0), 1.0, &cfg());
        assert_eq!(w.range_months, Some([6.0, 24.0]));
        assert_eq!(w.band.as_deref(), Some("late"));
    }

    #[test]
    fn analog_empty_when_no_score() {
        let w = analog_window(None, 0.0, &cfg());
        assert!(w.range_months.is_none());
        assert!(w.band.is_none());
    }
}
