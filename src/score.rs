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

/// One sub-question's score, computed over the SAME available-weight rule as the
/// composite.
///
/// `score` is `None` when none of the group's weight is available — the caller must
/// report "not measured", never 0. That distinction is the whole point: an unmeasured
/// question reading as calm is the failure this project exists to prevent.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubScore {
    pub key: String,
    /// Weighted mean over the group's AVAILABLE weight (renormalized within the group).
    pub score: Option<f64>,
    /// Weight the group's indicators carry in total, available or not.
    pub weight: f64,
    /// Weight actually available, so the reader can see how thin the group is.
    pub available_weight: f64,
    /// How many of the group's indicators produced a reading.
    pub measured: usize,
    pub total: usize,
    /// Composite-of-the-group over the group's TOTAL weight (available/total), so a
    /// group with a dead source shows reduced coverage instead of a confident score.
    pub coverage: f64,
}

/// Score each sub-question separately, over the groups named in `groups` (key, order).
///
/// WHY SEPARATE RE-COMPUTES RATHER THAN PARTITIONING THE COMPOSITE. Each group is
/// renormalized over its OWN available weight, exactly as the composite is. That is
/// deliberate: a group whose source is down must show reduced `coverage` within the
/// group and must not have its score propped up or dragged down by weight it never had.
///
/// Returns groups in `order`; groups with no members are omitted (config validation
/// refuses to start with such a group, so this is belt-and-braces).
pub fn sub_scores(readings: &[IndicatorReading], groups: &[(String, u32)]) -> Vec<SubScore> {
    let mut ordered = groups.to_vec();
    ordered.sort_by_key(|(_, o)| *o);

    let mut out = Vec::with_capacity(ordered.len());
    for (key, _) in ordered {
        let members: Vec<&IndicatorReading> = readings
            .iter()
            .filter(|r| r.group.as_deref() == Some(key.as_str()))
            .collect();
        if members.is_empty() {
            continue;
        }
        let total: f64 = members.iter().map(|r| r.weight).sum();
        let available: f64 = members
            .iter()
            .filter(|r| r.reading.is_available() && r.weight > 0.0)
            .map(|r| r.weight)
            .sum();
        let weighted: f64 = members
            .iter()
            .filter_map(|r| match (&r.reading, r.weight) {
                (Reading::Scored { stress, .. }, w) if w > 0.0 => Some(w * stress),
                _ => None,
            })
            .sum();
        let measured = members
            .iter()
            .filter(|r| r.reading.is_available() && r.weight > 0.0)
            .count();
        out.push(SubScore {
            key,
            score: if available > 0.0 {
                Some(weighted / available)
            } else {
                None
            },
            weight: total,
            available_weight: available,
            measured,
            total: members.iter().filter(|r| r.weight > 0.0).count(),
            coverage: if total > 0.0 { available / total } else { 0.0 },
        });
    }
    out
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
            group: None,
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
            group: None,
        }
    }

    fn grouped(id: &str, weight: f64, stress: f64, group: &str) -> IndicatorReading {
        let mut r = scored(id, weight, stress);
        r.group = Some(group.into());
        r
    }

    fn grouped_unavailable(id: &str, weight: f64, group: &str) -> IndicatorReading {
        let mut r = unavailable(id, weight);
        r.group = Some(group.into());
        r
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

    // ---------------------------------------------------------------- sub-scores

    #[test]
    fn a_sub_score_is_the_weighted_mean_of_its_own_group_only() {
        // Group A: (10@80, 10@20) -> 50.  Group B: (10@100) -> 100.
        // The whole point: neither group is dragged toward the other, which is
        // exactly what the blended composite does to them.
        let r = vec![
            grouped("a1", 10.0, 80.0, "A"),
            grouped("a2", 10.0, 20.0, "A"),
            grouped("b1", 10.0, 100.0, "B"),
        ];
        let g = vec![("A".to_string(), 1u32), ("B".to_string(), 2u32)];
        let out = sub_scores(&r, &g);
        assert_eq!(out.len(), 2);
        assert!((out[0].score.unwrap() - 50.0).abs() < 1e-9, "A must be 50");
        assert!(
            (out[1].score.unwrap() - 100.0).abs() < 1e-9,
            "B must be 100"
        );
        // And the blended composite is different from BOTH — that is the defect the
        // sub-scores exist to expose.
        let (blend, _) = composite(&r);
        assert!((blend.unwrap() - 66.66666666666667).abs() < 1e-6);
    }

    #[test]
    fn a_group_whose_source_is_down_reports_no_score_rather_than_zero() {
        // THE load-bearing distinction. An unmeasured question reading as "0" would
        // read as CALM — the opposite of the truth — and that is the failure mode
        // this project exists to prevent.
        let r = vec![
            grouped("a1", 10.0, 80.0, "A"),
            grouped_unavailable("b1", 10.0, "B"),
        ];
        let g = vec![("A".to_string(), 1u32), ("B".to_string(), 2u32)];
        let out = sub_scores(&r, &g);
        let b = out.iter().find(|s| s.key == "B").unwrap();
        assert!(
            b.score.is_none(),
            "a fully-dead group must report None, never 0.0"
        );
        assert_eq!(b.available_weight, 0.0);
        assert_eq!(b.coverage, 0.0);
        assert_eq!(b.measured, 0);
        assert_eq!(b.total, 1);
    }

    #[test]
    fn a_partly_dead_group_renormalizes_over_its_available_weight_only() {
        // Group A has 10@90 available and 10@0 unavailable. Renormalizing over
        // available weight gives 90, NOT 45 (=90*10/20). A dead source must shrink
        // the group's coverage, not halve its score.
        let r = vec![
            grouped("a1", 10.0, 90.0, "A"),
            grouped_unavailable("a2", 10.0, "A"),
        ];
        let g = vec![("A".to_string(), 1u32)];
        let out = sub_scores(&r, &g);
        assert!((out[0].score.unwrap() - 90.0).abs() < 1e-9);
        assert!((out[0].coverage - 0.5).abs() < 1e-9);
        assert_eq!(out[0].available_weight, 10.0);
        assert_eq!(out[0].weight, 20.0);
    }

    #[test]
    fn weight_zero_indicators_never_enter_a_sub_score() {
        // A declared gap (weight 0) is reported, never scored — the same rule the
        // composite follows.
        let r = vec![
            grouped("a1", 10.0, 50.0, "A"),
            grouped("gap", 0.0, 0.0, "A"),
        ];
        let g = vec![("A".to_string(), 1u32)];
        let out = sub_scores(&r, &g);
        assert!(
            (out[0].score.unwrap() - 50.0).abs() < 1e-9,
            "weight-0 must not pull it toward 0"
        );
        assert!((out[0].coverage - 1.0).abs() < 1e-9);
        assert_eq!(
            out[0].total, 1,
            "weight-0 is not counted as a scored member"
        );
    }

    #[test]
    fn sub_scores_follow_the_configured_order_not_the_input_order() {
        let r = vec![grouped("z", 10.0, 10.0, "Z"), grouped("a", 10.0, 20.0, "A")];
        // The GROUPS are declared deliberately OUT of order (Z has order 2 but is
        // listed first) — without the sort, the output would follow this declaration
        // order and the assertion below would fail. My first version of this test
        // passed the groups already sorted, so it could not fail and the falsifier
        // caught it.
        let g = vec![("Z".to_string(), 2u32), ("A".to_string(), 1u32)];
        let out = sub_scores(&r, &g);
        assert_eq!(out[0].key, "A");
        assert_eq!(out[1].key, "Z");
    }

    #[test]
    fn sub_scores_do_not_change_the_composite() {
        // The invariant this whole feature rests on. Adding groups to readings must
        // leave the composite byte-identical, exactly as the trend does.
        let bare = vec![scored("a", 10.0, 80.0), scored("b", 10.0, 20.0)];
        let with_groups = vec![grouped("a", 10.0, 80.0, "X"), grouped("b", 10.0, 20.0, "Y")];
        let (c1, v1) = composite(&bare);
        let (c2, v2) = composite(&with_groups);
        assert_eq!(
            c1, c2,
            "the composite must be byte-identical with and without groups"
        );
        assert_eq!(v1, v2);
    }

    #[test]
    fn an_unknown_group_is_omitted_rather_than_invented() {
        let r = vec![grouped("a", 10.0, 50.0, "A")];
        let g = vec![("A".to_string(), 1u32), ("ghost".to_string(), 2u32)];
        let out = sub_scores(&r, &g);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].key, "A");
    }
}
