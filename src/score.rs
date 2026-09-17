//! The pure scoring core.
//!
//! No IO, no clock, no randomness. Same inputs -> same outputs, always. This is
//! where the honesty contract is enforced:
//!
//!   * an unavailable indicator is NEVER imputed, defaulted, or scored;
//!   * the composite is renormalized over only the weight that is actually
//!     available, so a missing source shrinks `coverage` instead of dragging the
//!     score toward an arbitrary midpoint;
//!   * the contribution of every indicator is reported, so the number is fully
//!     reconstructable by hand.

use crate::model::{IndicatorReading, Reading};

/// Piecewise-linear interpolation over ascending raw-value anchors, with flat
/// (clamped) extrapolation beyond the outermost anchors.
///
/// Flat extrapolation is deliberate: an input far outside known history should
/// read as "extreme", not as an unbounded score that swamps everything else.
pub fn interpolate(raw: f64, anchors: &[[f64; 2]]) -> f64 {
    if anchors.is_empty() {
        return 0.0;
    }
    let mut a = anchors.to_vec();
    a.sort_by(|x, y| x[0].partial_cmp(&y[0]).unwrap());

    if raw <= a[0][0] {
        return a[0][1];
    }
    let last = a[a.len() - 1];
    if raw >= last[0] {
        return last[1];
    }
    for w in a.windows(2) {
        let (lo, hi) = (w[0], w[1]);
        if raw >= lo[0] && raw <= hi[0] {
            let span = hi[0] - lo[0];
            if span.abs() < f64::EPSILON {
                return hi[1];
            }
            let t = (raw - lo[0]) / span;
            return lo[1] + t * (hi[1] - lo[1]);
        }
    }
    last[1]
}

/// Weighted composite over available weight, plus the coverage fraction.
///
/// Returns `(composite, coverage)`. `composite` is `None` when nothing at all is
/// available — in which case the caller must report no score rather than 0.
///
/// Indicators with weight 0 (declared gaps) never affect either number.
pub fn composite(readings: &[IndicatorReading]) -> (Option<f64>, f64) {
    let total: f64 = readings.iter().map(|r| r.weight).sum();
    if total <= 0.0 {
        return (None, 0.0);
    }
    let available: f64 = readings
        .iter()
        .filter(|r| r.reading.is_available() && r.weight > 0.0)
        .map(|r| r.weight)
        .sum();

    if available <= 0.0 {
        return (None, 0.0);
    }

    let weighted: f64 = readings
        .iter()
        .filter_map(|r| match (&r.reading, r.weight) {
            (Reading::Scored { stress, .. }, w) if w > 0.0 => Some(w * stress),
            _ => None,
        })
        .sum();

    (Some(weighted / available), available / total)
}

/// Confidence band from coverage. Thresholds come from config, not from code.
pub fn confidence(coverage: f64, low_below: f64, high_above: f64) -> &'static str {
    if coverage < low_below {
        "low"
    } else if coverage > high_above {
        "high"
    } else {
        "medium"
    }
}

/// Fill in each indicator's renormalized contribution to the composite.
///
/// Keeping this separate from `composite` lets the report show, per indicator,
/// exactly how much of the final number it is responsible for.
pub fn attribute(readings: &mut [IndicatorReading]) {
    let available: f64 = readings
        .iter()
        .filter(|r| r.reading.is_available() && r.weight > 0.0)
        .map(|r| r.weight)
        .sum();
    for r in readings.iter_mut() {
        r.contribution = match (&r.reading, r.weight) {
            (Reading::Scored { stress, .. }, w) if w > 0.0 && available > 0.0 => {
                Some(w / available * stress)
            }
            _ => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Provenance;

    fn prov() -> Provenance {
        Provenance {
            source: "test".into(),
            endpoint: "test://".into(),
            as_of: "2026-09-16".into(),
            retrieved_at: "2026-09-16T00:00:00Z".into(),
        }
    }

    fn scored(id: &str, weight: f64, stress: f64) -> IndicatorReading {
        IndicatorReading {
            id: id.into(),
            label: id.into(),
            weight,
            rationale: String::new(),
            reading: Reading::Scored {
                stress,
                value: 0.0,
                unit: "u".into(),
                detail: String::new(),
                provenance: prov(),
            },
            contribution: None,
        }
    }

    fn unavailable(id: &str, weight: f64) -> IndicatorReading {
        IndicatorReading {
            id: id.into(),
            label: id.into(),
            weight,
            rationale: String::new(),
            reading: Reading::Unavailable {
                reason: "down".into(),
            },
            contribution: None,
        }
    }

    #[test]
    fn interpolation_hits_anchors_exactly() {
        let a = vec![[0.0, 0.0], [10.0, 50.0], [20.0, 100.0]];
        assert_eq!(interpolate(0.0, &a), 0.0);
        assert_eq!(interpolate(10.0, &a), 50.0);
        assert_eq!(interpolate(20.0, &a), 100.0);
    }

    #[test]
    fn interpolation_is_linear_between_anchors() {
        let a = vec![[0.0, 0.0], [10.0, 50.0]];
        assert!((interpolate(5.0, &a) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn interpolation_extrapolates_flat_not_unbounded() {
        let a = vec![[0.0, 10.0], [10.0, 90.0]];
        assert_eq!(interpolate(-1000.0, &a), 10.0);
        assert_eq!(interpolate(1e9, &a), 90.0);
    }

    #[test]
    fn interpolation_handles_unsorted_input() {
        let a = vec![[20.0, 100.0], [0.0, 0.0], [10.0, 50.0]];
        assert!((interpolate(15.0, &a) - 75.0).abs() < 1e-9);
    }

    #[test]
    fn composite_is_weighted_mean_when_all_available() {
        let r = vec![scored("a", 10.0, 100.0), scored("b", 10.0, 0.0)];
        let (c, cov) = composite(&r);
        assert!((c.unwrap() - 50.0).abs() < 1e-9);
        assert!((cov - 1.0).abs() < 1e-9);
    }

    #[test]
    fn missing_indicator_shrinks_coverage_and_does_not_drag_score() {
        // Two indicators: one stressed, one missing. Renormalizing over the
        // available weight must NOT pull the composite toward zero or fifty.
        let r = vec![scored("a", 10.0, 80.0), unavailable("b", 10.0)];
        let (c, cov) = composite(&r);
        assert!((c.unwrap() - 80.0).abs() < 1e-9, "composite must not drift");
        assert!((cov - 0.5).abs() < 1e-9);
    }

    #[test]
    fn declared_gap_weight_zero_does_not_affect_score_or_coverage() {
        let r = vec![scored("a", 10.0, 60.0), unavailable("gap", 0.0)];
        let (c, cov) = composite(&r);
        assert!((c.unwrap() - 60.0).abs() < 1e-9);
        assert!((cov - 1.0).abs() < 1e-9);
    }

    #[test]
    fn no_data_yields_no_score_rather_than_zero() {
        let r = vec![unavailable("a", 10.0), unavailable("b", 5.0)];
        let (c, cov) = composite(&r);
        assert!(c.is_none(), "must not invent a score");
        assert_eq!(cov, 0.0);
    }

    #[test]
    fn contributions_sum_to_composite() {
        let mut r = vec![scored("a", 30.0, 40.0), scored("b", 10.0, 80.0)];
        attribute(&mut r);
        let sum: f64 = r.iter().filter_map(|x| x.contribution).sum();
        let (c, _) = composite(&r);
        assert!(
            (sum - c.unwrap()).abs() < 1e-9,
            "contributions must reconstruct the composite"
        );
    }

    #[test]
    fn unavailable_indicators_have_no_contribution() {
        let mut r = vec![scored("a", 10.0, 50.0), unavailable("b", 10.0)];
        attribute(&mut r);
        assert!(r[1].contribution.is_none());
    }

    #[test]
    fn confidence_bands_follow_config() {
        assert_eq!(confidence(0.5, 0.6, 0.85), "low");
        assert_eq!(confidence(0.7, 0.6, 0.85), "medium");
        assert_eq!(confidence(0.9, 0.6, 0.85), "high");
    }

    #[test]
    fn scoring_is_deterministic() {
        let mk = || vec![scored("a", 10.0, 33.3), scored("b", 20.0, 66.6)];
        let a = composite(&mk());
        let b = composite(&mk());
        assert_eq!(a, b);
    }
}
