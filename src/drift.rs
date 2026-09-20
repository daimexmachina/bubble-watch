//! Attributing composite change to the MODEL versus the MARKET.
//!
//! THE PROBLEM THIS EXISTS TO SOLVE. The run archive shows the composite climbing from 30.6 to
//! 43.0 over one session. That looks like deteriorating conditions. It is almost entirely the
//! model being modified: measured across the archive, methodology changes account for **+10.1**
//! points while within-methodology movement averages **0.89**. Roughly nine tenths of the rise is
//! the INSTRUMENT changing, not the thing it measures.
//!
//! That is a defect in how the tool reports its own history, not in the indicators. A composite
//! that can be raised by editing the model, and that reports no distinction between the two, is
//! one a reader will misread by default — and the reader would be reasonable to do so, because
//! nothing in the output says otherwise.
//!
//! WHAT THIS MODULE DOES. It splits archived change into two attributable parts:
//!
//!   * MODEL change — the composite differs because the methodology differs. Nothing can be
//!     concluded about the market from it.
//!   * MARKET movement — the composite differs across dates under a SINGLE methodology. This is
//!     the only part that says anything about the world.
//!
//! Both are reported, always together, so neither can be read alone. A large model component is
//! stated as a caveat on any apparent trend rather than left for the reader to notice.
//!
//! WHY NOT JUST STOP CHANGING THE MODEL. Because adding indicators is how the tool improves, and
//! refusing to improve it to keep a number stable would be worse. The honest answer is not to
//! freeze the model but to be explicit about which part of the movement it caused.

use crate::model::TrendPoint;
use serde::{Deserialize, Serialize};

/// One methodology's contribution to the composite's history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MethodEpoch {
    pub methodology: String,
    /// First composite recorded under this methodology.
    pub first_composite: f64,
    /// Last composite recorded under it.
    pub last_composite: f64,
    /// Highest and lowest seen, all under this methodology.
    pub min_composite: f64,
    pub max_composite: f64,
    /// Number of runs recorded.
    pub runs: usize,
    /// Distinct calendar dates.
    pub dates: usize,
}

impl MethodEpoch {
    /// Movement WITHIN this methodology — the only part attributable to the market.
    ///
    /// Reported as the RANGE rather than first-to-last because with few observations the two
    /// coincide, and the range is the honest bound on how much the market could have moved.
    pub fn market_range(&self) -> f64 {
        self.max_composite - self.min_composite
    }
}

/// The full attribution, separating model change from market movement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriftAttribution {
    /// Per-methodology epochs, ordered by the sequence they first appear.
    pub epochs: Vec<MethodEpoch>,
    /// Sum of first-composite jumps BETWEEN consecutive methodologies. Model-caused.
    pub model_change: f64,
    /// Mean range within a methodology. Market-caused, and the only part that speaks to the world.
    pub mean_market_range: f64,
    /// True when there is at least one methodology with more than one recorded date, i.e. when any
    /// market movement can be measured at all.
    pub market_measurable: bool,
}

impl DriftAttribution {
    /// The share of total movement caused by changing the model, or None when no movement occurred.
    pub fn model_share(&self) -> Option<f64> {
        let total = self.model_change.abs() + self.mean_market_range;
        if total <= f64::EPSILON {
            return None;
        }
        Some(self.model_change.abs() / total)
    }

    /// A plain statement of the split, for the report. Deliberately blunt: this is the sentence a
    /// reader must not be able to miss, because the failure mode is reading a model change as a
    /// market signal.
    pub fn statement(&self) -> String {
        if self.epochs.is_empty() {
            return "No run archive, so no attribution is possible.".to_string();
        }
        let share = match self.model_share() {
            Some(s) => format!("{:.0}%", s * 100.0),
            None => "n/a".to_string(),
        };
        let market_note = if self.market_measurable {
            format!(
                "Market movement within a single methodology averages {:.2} points.",
                self.mean_market_range
            )
        } else {
            "NO market movement is measurable yet: every methodology so far has been recorded on \
             only one date, so any change observed has been a MODEL change and none of it can be \
             attributed to the market."
                .to_string()
        };
        format!(
            "COMPOSITE HISTORY ATTRIBUTION. Across {} methodology version(s), changes to the MODEL \
             account for {:+.1} points of movement; {} Model-caused movement is roughly {} of the \
             total. A reader comparing two composites from different methodology versions is \
             comparing two DIFFERENT INSTRUMENTS, not one instrument at two times, and the tool \
             refuses such a comparison everywhere else for that reason.",
            self.epochs.len(),
            self.model_change,
            market_note,
            share,
        )
    }
}

/// Build the attribution from an archive. Pure.
///
/// Epochs are ordered by FIRST APPEARANCE in the archive, which is the order they were adopted.
pub fn attribute(archive: &[TrendPoint]) -> DriftAttribution {
    // Preserve adoption order rather than sorting: methodology "1.10" would sort before "1.2" as a
    // string, and adoption order is the meaningful sequence anyway.
    let mut order: Vec<String> = Vec::new();
    let mut by_method: std::collections::BTreeMap<String, Vec<&TrendPoint>> =
        std::collections::BTreeMap::new();
    for p in archive {
        let m = crate::history::methodology_of(p).to_string();
        if !order.contains(&m) {
            order.push(m.clone());
        }
        by_method.entry(m).or_default().push(p);
    }

    let mut epochs = Vec::new();
    for m in &order {
        let pts = &by_method[m];
        if pts.is_empty() {
            continue;
        }
        let composites: Vec<f64> = pts.iter().map(|p| p.composite).collect();
        let mut dates: Vec<&str> = pts.iter().map(|p| p.date.as_str()).collect();
        dates.sort();
        dates.dedup();
        epochs.push(MethodEpoch {
            methodology: m.clone(),
            first_composite: composites[0],
            last_composite: *composites.last().unwrap_or(&composites[0]),
            min_composite: composites.iter().cloned().fold(f64::INFINITY, f64::min),
            max_composite: composites.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            runs: pts.len(),
            dates: dates.len(),
        });
    }

    let model_change: f64 = epochs
        .windows(2)
        .map(|w| w[1].first_composite - w[0].first_composite)
        .sum();

    let multi: Vec<&MethodEpoch> = epochs.iter().filter(|e| e.dates > 1).collect();
    let mean_market_range = if multi.is_empty() {
        0.0
    } else {
        multi.iter().map(|e| e.market_range()).sum::<f64>() / multi.len() as f64
    };

    DriftAttribution {
        market_measurable: !multi.is_empty(),
        epochs,
        model_change,
        mean_market_range,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn pt(date: &str, composite: f64, method: &str) -> TrendPoint {
        TrendPoint {
            date: date.into(),
            generated_at: format!("{}T12:00:00Z", date),
            composite,
            coverage: 1.0,
            phase: "mid".into(),
            methodology_version: method.into(),
            stresses: BTreeMap::new(),
        }
    }

    #[test]
    fn a_model_change_is_attributed_to_the_model_not_the_market() {
        // Two methodologies on one date each: all movement is model-caused, and the report must
        // say that NO market movement is measurable.
        let a = vec![pt("2026-09-01", 30.0, "1.0"), pt("2026-09-02", 42.0, "2.0")];
        let d = attribute(&a);
        assert!(
            (d.model_change - 12.0).abs() < 1e-9,
            "got {}",
            d.model_change
        );
        assert!(!d.market_measurable, "no methodology spans two dates");
        assert_eq!(d.mean_market_range, 0.0);
        assert!(d.statement().contains("NO market movement is measurable"));
        // And the model share is 100%, because there is no market component at all.
        assert!((d.model_share().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn market_movement_is_measured_only_within_one_methodology() {
        // Same methodology across two dates: that IS market movement and must be counted.
        let a = vec![pt("2026-09-01", 40.0, "2.0"), pt("2026-09-02", 41.5, "2.0")];
        let d = attribute(&a);
        assert!(d.market_measurable);
        assert!((d.mean_market_range - 1.5).abs() < 1e-9);
        assert_eq!(d.model_change, 0.0, "no methodology changed");
        assert!(
            (d.model_share().unwrap()).abs() < 1e-9,
            "all movement is market"
        );
    }

    #[test]
    fn the_split_reports_both_sides_and_never_one_alone() {
        // The failure this module prevents is a reader seeing "+12 since the model started" and
        // reading it as the market. Both numbers must always appear together.
        let a = vec![
            pt("2026-09-01", 30.0, "1.0"),
            pt("2026-09-02", 32.0, "1.0"),
            pt("2026-09-03", 45.0, "2.0"),
            pt("2026-09-04", 45.5, "2.0"),
        ];
        let d = attribute(&a);
        let s = d.statement();
        assert!(s.contains("MODEL"), "must name the model component: {}", s);
        assert!(
            s.contains("Market movement"),
            "must name the market component: {}",
            s
        );
        assert!(
            s.contains("DIFFERENT INSTRUMENTS"),
            "must state the incomparability explicitly: {}",
            s
        );
        // With +15 model and +2 market, the model dominates and the share should say so.
        assert!(
            d.model_share().unwrap() > 0.8,
            "share was {:?}",
            d.model_share()
        );
    }

    #[test]
    fn an_empty_archive_produces_no_claim_rather_than_a_zero() {
        let d = attribute(&[]);
        assert!(d.epochs.is_empty());
        assert!(
            d.model_share().is_none(),
            "no movement means no share, not a zero share"
        );
        assert!(d.statement().contains("no attribution is possible"));
    }
}
