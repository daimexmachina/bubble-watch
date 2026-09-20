//! The frontier premium: how much better the best closed model is than the best open one.
//!
//! WHY THIS MEASURES SOMETHING NO PREVIOUS BUBBLE HAD. In every prior technology mania
//! the thing being financed could not be reproduced by anyone else. Railways needed
//! land, telecom needed spectrum, the dot-com buildout needed proprietary code and a
//! sales force. Here the core asset — a frontier language model — is contested by an
//! open ecosystem that publishes weights, and the gap between the best closed model and
//! the best open one is SMALL and MEASURABLE. If that gap closes, the revenue that is
//! meant to justify hundreds of billions of capital spending loses its moat.
//!
//! Measured from this host 2026-09-19, from 243 LMArena leaderboard snapshots:
//!
//!   frontier gap (best proprietary rating minus best non-proprietary)
//!     2024-02-02   +135.0   <- peak
//!     2024-09-17    +89.2
//!     2025-01-24     -5.3   <- briefly NEGATIVE: open models ahead
//!     2025-06-24    +54.0
//!     2026-04-02    +25.0
//!     2026-09-13    +32.5   <- latest
//!
//! The premium compressed from ~135 Elo to ~32, with a period in early 2025 where the
//! best open model outranked the best closed one outright.
//!
//! DIRECTION — AND THIS IS THE SUBTLE PART. A NARROW gap is the stress signal, which is
//! the OPPOSITE convention to most indicators in this model. A wide gap means the
//! frontier is genuinely far ahead and the pricing power is earned; a narrow gap means
//! the moat is thin while the spending assumes it is wide. So this indicator INVERTS:
//! high stress = small gap.
//!
//! WHY IT IS ONLY A PARTIAL MEASURE, stated in the output:
//!
//!   * Arena ratings measure PREFERENCE on human-voted prompts, not the enterprise
//!     workloads that actually generate the revenue. A model can win the arena and be
//!     unusable in production;
//!   * a licence labelled "Proprietary" is not the same as "weights unreleased" — the
//!     split here follows the leaderboard's own licence field, which is the best
//!     available and is not the same thing;
//!   * the gap is between the SINGLE best model on each side, so one release moves it
//!     sharply. It is a margin, not a distribution.
//!
//! It is also genuinely two-sided: a NARROW gap is bad for the incumbents financing the
//! buildout, but it is good for AI adoption generally. The output says so rather than
//! pretending the number has one meaning.

use crate::model::Point;

/// One leaderboard snapshot of the frontier gap.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GapPoint {
    pub date: String,
    pub best_proprietary: f64,
    pub best_open: f64,
    pub gap: f64,
    /// How many proprietary (closed) models the leaderboard ranked on this date. The
    /// comparison is only meaningful above `MIN_PROPRIETARY_MODELS`.
    pub n_proprietary: usize,
    /// How many open-weight models were ranked, recorded for the same reason.
    pub n_open: usize,
    pub best_proprietary_model: String,
    pub best_open_model: String,
}

/// Minimum number of ranked proprietary models for a snapshot to be a usable comparison.
///
/// WHY THIS EXISTS. The series is the best closed model against the best open one, so it is
/// only a capability comparison when BOTH sides are properly represented. In LMArena's early
/// snapshots they were not: from 2023-05 to 2023-11 the leaderboard ranked exactly ONE
/// proprietary model (`palm-2`) against up to 30 open-weight entries. The resulting gap fell
/// to −102.0, which reads as "open models led by a mile" but actually measures leaderboard
/// composition — closed labs had barely entered the arena yet.
///
/// The tell is that the gap jumps −102.0 → +65.0 between 2023-11-16 and 2023-12-06, precisely
/// as proprietary entries appear (1 → 4 → 9). No capability discontinuity can do that.
///
/// Admitting those snapshots would bias the headline in the ALARMING direction, which is the
/// direction this project is most careful about. The threshold is applied to the SERIES, not
/// only to the prose, so the guarded reading cannot drift back in through a later change.
pub const MIN_PROPRIETARY_MODELS: usize = 5;

/// The comparable subset: snapshots where the closed side is represented well enough for the
/// difference to mean something. See `MIN_PROPRIETARY_MODELS` for why this is not cosmetic.
pub fn comparable(points: &[GapPoint]) -> Vec<GapPoint> {
    points
        .iter()
        .filter(|p| p.n_proprietary >= MIN_PROPRIETARY_MODELS)
        .cloned()
        .collect()
}

/// Load the committed gap series. Returns None when the fixture is absent, so the
/// indicator reports an honest gap rather than a fabricated number.
///
/// The returned series is the FULL recorded history; callers that are computing a
/// capability comparison must pass it through `comparable` first. `load` stays complete so
/// the artifact remains inspectable rather than hidden — the guard belongs at the point of
/// interpretation, where its reasoning is visible.
pub fn load() -> Option<Vec<GapPoint>> {
    let path = std::path::Path::new("tests/fixtures/arena_frontier_gap.json");
    let text = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut out = Vec::new();
    for s in v.get("series")?.as_array()? {
        out.push(GapPoint {
            date: s.get("date")?.as_str()?.to_string(),
            best_proprietary: s.get("best_proprietary")?.as_f64()?,
            best_open: s.get("best_open")?.as_f64()?,
            gap: s.get("gap")?.as_f64()?,
            // Counts default to 0 when absent, which EXCLUDES the snapshot rather than
            // admitting an unverifiable comparison.
            n_proprietary: s.get("n_proprietary").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            n_open: s.get("n_open").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
            best_proprietary_model: s
                .get("best_proprietary_model")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            best_open_model: s
                .get("best_open_model")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// The gap as a `Point` series, so it can flow through the same plumbing as every other
/// retrieved datum.
pub fn as_points(points: &[GapPoint]) -> Vec<Point> {
    points
        .iter()
        .map(|g| Point {
            date: g.date.clone(),
            value: g.gap,
        })
        .collect()
}

/// The peak gap in the series, and when it occurred. The reference point that makes the
/// current reading interpretable — a gap of 32 means nothing without knowing it was once
/// 135.
pub fn peak(points: &[GapPoint]) -> Option<&GapPoint> {
    points.iter().max_by(|a, b| {
        a.gap
            .partial_cmp(&b.gap)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Compression from the peak, as a percentage. Higher = the premium has eroded more.
pub fn compression_from_peak_pct(points: &[GapPoint]) -> Option<f64> {
    let pk = peak(points)?;
    let last = points.last()?;
    if pk.gap <= 0.0 {
        return None;
    }
    Some((1.0 - last.gap / pk.gap) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_loads_with_the_full_history() {
        let Some(p) = load() else {
            eprintln!("SKIP: arena fixture absent");
            return;
        };
        assert!(p.len() > 200, "expected ~243 snapshots, got {}", p.len());
        assert!(p[0].date < p[p.len() - 1].date, "must be chronological");
        for g in &p {
            assert!(
                (g.best_proprietary - g.best_open - g.gap).abs() < 0.15,
                "gap must equal the difference: {} vs {}",
                g.gap,
                g.best_proprietary - g.best_open
            );
        }
    }

    #[test]
    fn the_premium_has_compressed_materially() {
        // The headline claim. If this ever stops holding, the commoditization thesis
        // needs revisiting rather than the test being relaxed.
        let Some(p) = load() else {
            eprintln!("SKIP: arena fixture absent");
            return;
        };
        let c = compression_from_peak_pct(&p).expect("peak gap is positive");
        assert!(
            c > 50.0,
            "the frontier premium should have compressed by more than half, got {:.1}%",
            c
        );
    }

    #[test]
    fn the_gap_once_went_negative() {
        // A real feature of the history that a naive "gap is positive" assumption would
        // have broken on, and worth pinning: open models briefly led outright.
        let Some(p) = load() else {
            eprintln!("SKIP: arena fixture absent");
            return;
        };
        assert!(
            p.iter().any(|g| g.gap < 0.0),
            "the series contains a period where the best open model outranked the best closed one"
        );
    }

    #[test]
    fn peak_identifies_the_widest_premium() {
        let Some(p) = load() else {
            eprintln!("SKIP: arena fixture absent");
            return;
        };
        let pk = peak(&p).unwrap();
        assert!(
            pk.gap > 100.0,
            "peak gap should exceed 100 Elo, got {}",
            pk.gap
        );
        assert!(
            p.iter().all(|g| g.gap <= pk.gap + 1e-9),
            "no point may exceed the reported peak"
        );
    }

    #[test]
    fn staleness_is_measurable_from_the_series_itself() {
        // The guard in the indicator needs to know the age of the newest snapshot
        // relative to the run. That arithmetic lives here so it is testable without
        // constructing a whole Observations.
        let p = vec![GapPoint {
            date: "2026-09-13".into(),
            best_proprietary: 1507.6,
            best_open: 1475.1,
            gap: 32.5,
            n_proprietary: 180,
            n_open: 180,
            best_proprietary_model: "a".into(),
            best_open_model: "b".into(),
        }];
        let last = p.last().unwrap();
        // 120-day limit: fresh on the day, stale well past it
        assert!(crate::sources::edgar::days_between(&last.date, "2026-09-19") < 120);
        assert!(crate::sources::edgar::days_between(&last.date, "2027-06-01") > 120);
    }

    #[test]
    fn points_convert_for_the_shared_plumbing() {
        let g = vec![GapPoint {
            date: "2026-09-13".into(),
            best_proprietary: 1507.6,
            best_open: 1475.1,
            gap: 32.5,
            n_proprietary: 180,
            n_open: 180,
            best_proprietary_model: "a".into(),
            best_open_model: "b".into(),
        }];
        let pts = as_points(&g);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].value - 32.5).abs() < 1e-9);
    }

    #[test]
    fn thin_snapshots_are_excluded_from_the_comparison() {
        // The guard that keeps a leaderboard artifact out of a capability reading. In
        // 2023-05..2023-11 LMArena ranked ONE proprietary model against up to 30 open ones,
        // and the resulting -102 gap would read as "open models led by a mile" when it
        // actually measures that closed labs had barely entered the arena.
        let mk = |date: &str, gap: f64, np: usize| GapPoint {
            date: date.into(),
            best_proprietary: 1000.0,
            best_open: 1000.0 + gap,
            gap,
            n_proprietary: np,
            n_open: 30,
            best_proprietary_model: "p".into(),
            best_open_model: "o".into(),
        };
        let raw = vec![
            mk("2023-05-22", -16.0, 1),
            mk("2023-11-16", -102.0, 1),
            mk("2023-12-15", 75.3, 5),
            mk("2026-09-13", 32.5, 180),
        ];
        let c = comparable(&raw);
        assert_eq!(c.len(), 2, "only well-populated snapshots survive");
        assert!(
            c.iter().all(|p| p.n_proprietary >= MIN_PROPRIETARY_MODELS),
            "every survivor must clear the threshold"
        );
        // The artifact's -102 must NOT be reachable through the comparable view, because a
        // headline built on it would be biased toward alarm.
        assert!(
            !c.iter().any(|p| p.gap < -90.0),
            "the -102 leaderboard artifact must not survive the filter"
        );
    }

    #[test]
    fn the_real_fixture_keeps_its_measured_negative_episode() {
        // THE COMPLEMENT, and the reason the threshold is 5 rather than something higher:
        // filtering must remove the artifact WITHOUT erasing the one genuinely measured
        // negative episode. In 2025-01-24..2025-02-27 deepseek-r1 really did outrank o1 and
        // gemini-2.0-flash, against 45-55 closed and 114 open models. A guard that also
        // deleted that would hide a real fact of the same shape as the artifact.
        let Some(raw) = load() else {
            eprintln!("SKIP: fixture absent");
            return;
        };
        let c = comparable(&raw);
        let neg: Vec<_> = c.iter().filter(|p| p.gap < 0.0).collect();
        assert!(
            !neg.is_empty(),
            "the measured 2025 negative episode must survive the filter"
        );
        assert!(
            neg.iter().all(|p| p.date.starts_with("2025-")),
            "and it must be the 2025 episode, not the early artifact: {:?}",
            neg.iter().map(|p| &p.date).collect::<Vec<_>>()
        );
        assert!(
            c.iter().all(|p| p.n_proprietary >= MIN_PROPRIETARY_MODELS),
            "and the comparable view must be internally consistent"
        );
        // The excluded ones are exactly the thin early snapshots.
        let excluded: Vec<_> = raw
            .iter()
            .filter(|p| p.n_proprietary < MIN_PROPRIETARY_MODELS)
            .collect();
        assert!(
            excluded.iter().all(|p| p.date.as_str() < "2024-01-01"),
            "only early thin snapshots should be excluded: {:?}",
            excluded.iter().map(|p| &p.date).collect::<Vec<_>>()
        );
    }
}
