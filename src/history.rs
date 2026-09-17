//! Run history and direction of travel.
//!
//! Why this module exists: the tool's own credit rationale says the informative
//! signal is the DIRECTION OF TRAVEL, not the level. A point-in-time scorer
//! cannot see direction, because nothing persists between runs.
//!
//! Three rules govern everything here.
//!
//! 1. **The trend never enters the composite.** It is derived FROM the composite,
//!    so scoring it inside the composite would make the score partly a function
//!    of its own past. Adding it as an indicator would also silently change a
//!    published, audited number. `report::build` is required to produce a
//!    byte-identical composite with or without history, and a test asserts it.
//!
//! 2. **Comparing runs of unequal coverage is invalid, so it is refused.** The
//!    composite is renormalized over available weight (see `score.rs`), so a run
//!    at 100% coverage and one at 62% are not on the same scale; their
//!    difference is partly an artefact of which sources answered. Unless a
//!    baseline exists within the configured coverage tolerance, NO delta is
//!    reported and the reason is printed instead. There is deliberately no
//!    "use the previous run anyway" fallback — that would manufacture a
//!    directional claim out of a definitional difference.
//!
//! 3. **A delta is never shown without its elapsed time.** A +4 change over 3
//!    days and a +4 change over 400 days are different facts.
//!
//! The archive is append-only, so nothing is ever destroyed; the *series* used
//! for display keeps at most one point per calendar date, so re-running the tool
//! three times in an afternoon cannot masquerade as three days of history.

use crate::config::TrendCfg;
use crate::model::{
    Direction, IndicatorReading, IndicatorTrend, Reading, Trend, TrendDelta, TrendPoint,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Default archive location, relative to the working directory.
pub const DEFAULT_HISTORY_DIR: &str = "data/history";
pub const ARCHIVE_FILE: &str = "runs.jsonl";

pub fn archive_path(dir: &Path) -> PathBuf {
    dir.join(ARCHIVE_FILE)
}

/// Where per-run report pages are archived, so the site can link to the full
/// report for any past day rather than only the newest one.
pub fn reports_dir(dir: &Path) -> PathBuf {
    dir.join("reports")
}

/// Persist one run's rendered report page. Called when a run is recorded, so
/// that every archived date has a page behind it and the dashboard never links
/// to a file that does not exist.
pub fn save_report_html(dir: &Path, date: &str, html: &str) -> Result<PathBuf, String> {
    let rd = reports_dir(dir);
    std::fs::create_dir_all(&rd).map_err(|e| format!("cannot create {}: {}", rd.display(), e))?;
    let path = rd.join(format!("{}.html", date));
    std::fs::write(&path, html).map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
    Ok(path)
}

/// Reduce a report's current state to the record that gets archived.
///
/// Unavailable indicators are simply absent from `stresses` — never written as
/// 0.0. Zero is a legitimate stress reading, so defaulting a gap to zero would
/// corrupt the series in a way that looks like real data.
pub fn point_from(
    readings: &[IndicatorReading],
    composite: f64,
    coverage: f64,
    phase: &str,
    generated_at: &str,
) -> TrendPoint {
    let mut stresses = BTreeMap::new();
    for r in readings {
        if r.weight <= 0.0 {
            continue; // declared blind spot, never scored
        }
        if let Reading::Scored { stress, .. } = &r.reading {
            stresses.insert(r.id.clone(), *stress);
        }
    }
    TrendPoint {
        date: date_of(generated_at),
        generated_at: generated_at.to_string(),
        composite,
        coverage,
        phase: phase.to_string(),
        stresses,
    }
}

/// Extract the YYYY-MM-DD date from an ISO-8601 timestamp, defensively: any
/// string that does not start with a plausible date falls back to the whole
/// string, so a malformed timestamp degrades rather than panics.
pub fn date_of(timestamp: &str) -> String {
    let head = timestamp.split('T').next().unwrap_or(timestamp);
    if head.len() == 10 && head.as_bytes()[4] == b'-' && head.as_bytes()[7] == b'-' {
        head.to_string()
    } else {
        head.to_string()
    }
}

/// Parse one JSONL line into a point, returning a warning string on failure.
fn parse_line(idx: usize, line: &str) -> Result<TrendPoint, String> {
    serde_json::from_str::<TrendPoint>(line).map_err(|e| {
        format!(
            "archive line {} is malformed and was skipped: {}",
            idx + 1,
            e
        )
    })
}

/// Load the archive. Malformed lines are reported as warnings and skipped, so
/// one bad byte neither destroys the history nor passes silently.
pub fn load(dir: &Path) -> (Vec<TrendPoint>, Vec<String>) {
    let path = archive_path(dir);
    let mut warnings = Vec::new();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return (Vec::new(), warnings);
    };
    let mut points = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_line(i, line) {
            Ok(p) => points.push(p),
            Err(w) => warnings.push(w),
        }
    }
    (points, warnings)
}

/// Collapse an append-only archive to at most one point per calendar date,
/// keeping the LAST entry written for each date (the most recent state of that
/// day), ordered oldest first.
///
/// This is a selection rule for display, not a deletion: the archive keeps
/// every run.
pub fn daily_series(points: &[TrendPoint]) -> Vec<TrendPoint> {
    let mut by_date: BTreeMap<&str, &TrendPoint> = BTreeMap::new();
    for p in points {
        // Later entries overwrite earlier ones for the same date.
        by_date.insert(p.date.as_str(), p);
    }
    by_date.into_values().cloned().collect()
}

/// Append one run to the archive, creating the directory if needed.
///
/// Append-only by design: a re-run is part of the audit trail, not an edit.
pub fn append(dir: &Path, point: &TrendPoint) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {}", dir.display(), e))?;
    let path = archive_path(dir);
    let line = serde_json::to_string(point).map_err(|e| format!("cannot serialise run: {}", e))?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open {}: {}", path.display(), e))?;
    writeln!(f, "{}", line).map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
    Ok(())
}

/// Whole days between two YYYY-MM-DD dates, or None if either is unparseable.
///
/// Returns a signed value: positive when `to` is later than `from`.
pub fn days_between(from: &str, to: &str) -> Option<f64> {
    let f = parse_date(from)?;
    let t = parse_date(to)?;
    Some((t - f).num_seconds() as f64 / 86_400.0)
}

fn parse_date(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Is this archived point usable as a baseline for the current run?
///
/// Both conditions are refusals, not preferences:
///   * the elapsed gap must reach `min_gap_days`, or a re-run minutes later
///     would be presented as a trend;
///   * weighted coverage must be within `coverage_tolerance_pp` of the current
///     run, or the difference partly reflects which sources answered rather than
///     a change in the market.
pub fn is_eligible(
    candidate: &TrendPoint,
    current_coverage: f64,
    today: &str,
    t: &TrendCfg,
) -> Result<f64, String> {
    // A run from the same day is never its own baseline, regardless of how
    // `min_gap_days` is configured: comparing today with today would report
    // 0.0 as if it were a measurement of market change.
    if candidate.date == today {
        return Err(format!(
            "the most recent archive entry is from today, below the {} day minimum gap",
            fmt_days(t.min_gap_days)
        ));
    }
    let gap = days_between(&candidate.date, today)
        .ok_or_else(|| format!("baseline date '{}' is unparseable", candidate.date))?;
    if gap < t.min_gap_days {
        return Err(format!(
            "most recent baseline is {} day(s) old, below the {} day minimum",
            fmt_days(gap),
            fmt_days(t.min_gap_days)
        ));
    }
    let diff_pp = (candidate.coverage - current_coverage).abs() * 100.0;
    if diff_pp > t.coverage_tolerance_pp {
        return Err(format!(
            "coverage differs by {:.1}pp (baseline {:.0}% vs current {:.0}%), above the {:.1}pp \
             tolerance — the two runs are not on the same scale",
            diff_pp,
            candidate.coverage * 100.0,
            current_coverage * 100.0,
            t.coverage_tolerance_pp
        ));
    }
    Ok(gap)
}

fn fmt_days(d: f64) -> String {
    if (d - d.round()).abs() < 1e-9 {
        format!("{}", d.round() as i64)
    } else {
        format!("{:.1}", d)
    }
}

/// Compute the full trend: the daily series, plus a delta against the most
/// recent ELIGIBLE baseline, plus an explicit reason when there is none.
///
/// PURE — no IO, no clock. `today` is passed in, so this is fully testable and
/// deterministic.
pub fn compute(
    archive: &[TrendPoint],
    current: &TrendPoint,
    readings: &[IndicatorReading],
    t: &TrendCfg,
    recorded: bool,
    warnings: Vec<String>,
) -> Trend {
    // The displayed series includes today's run.
    let mut all = archive.to_vec();
    all.push(current.clone());
    let points = daily_series(&all);

    // Candidate baselines: every prior archived entry, newest first. Entries
    // from today are included so the refusal message can be accurate ("the most
    // recent entry is from today") rather than falsely claiming there is no
    // previous run at all.
    let mut candidates: Vec<&TrendPoint> = archive.iter().collect();
    candidates.sort_by(|a, b| a.date.cmp(&b.date));

    // Walk newest-first, taking the most recent baseline that satisfies BOTH
    // the gap and the coverage rule. Older runs are considered so that a run
    // recorded during a source outage does not permanently blind the feature —
    // but each one still has to pass the coverage test to be used.
    let mut rejections: Vec<String> = Vec::new();
    for c in candidates.iter().rev() {
        match is_eligible(c, current.coverage, &current.date, t) {
            Ok(gap) => {
                let delta = build_delta(c, current, readings, gap, t);
                return Trend {
                    points,
                    delta: Some(delta),
                    reason: None,
                    warnings,
                    recorded,
                };
            }
            Err(why) => rejections.push(why),
        }
    }

    let reason = if candidates.is_empty() {
        "No previous run is recorded yet, so direction of travel cannot be computed. This is \
         expected on a first run. Run the tool again on a later day and a comparison will appear."
            .to_string()
    } else {
        format!(
            "No eligible baseline run: {}. A direction of travel is deliberately NOT reported \
             rather than computed against a run that is not comparable.",
            rejections.join("; ")
        )
    };

    Trend {
        points,
        delta: None,
        reason: Some(reason),
        warnings,
        recorded,
    }
}

fn build_delta(
    baseline: &TrendPoint,
    current: &TrendPoint,
    readings: &[IndicatorReading],
    elapsed_days: f64,
    t: &TrendCfg,
) -> TrendDelta {
    let composite_delta = current.composite - baseline.composite;

    // Per-indicator changes, only where BOTH runs scored that indicator.
    // Comparing a number against a gap would be comparing a number with a
    // non-number, so a gap on either side means the indicator is omitted here
    // (it is still reported as a gap in the main indicator table).
    let mut indicators = Vec::new();
    for r in readings {
        if r.weight <= 0.0 {
            continue;
        }
        let Reading::Scored {
            stress: current_stress,
            ..
        } = &r.reading
        else {
            continue;
        };
        let Some(baseline_stress) = baseline.stresses.get(&r.id) else {
            continue;
        };
        let delta = current_stress - baseline_stress;
        indicators.push(IndicatorTrend {
            id: r.id.clone(),
            label: r.label.clone(),
            current_stress: *current_stress,
            baseline_stress: *baseline_stress,
            delta,
            direction: Direction::classify(delta, t.flat_band),
        });
    }
    // Largest absolute movement first, so the reader sees what changed most.
    indicators.sort_by(|a, b| {
        b.delta
            .abs()
            .partial_cmp(&a.delta.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    TrendDelta {
        baseline_date: baseline.date.clone(),
        baseline_composite: baseline.composite,
        baseline_coverage: baseline.coverage,
        composite_delta,
        elapsed_days,
        direction: Direction::classify(composite_delta, t.flat_band),
        phase_now: current.phase.clone(),
        phase_then: baseline.phase.clone(),
        phase_changed: current.phase != baseline.phase,
        indicators,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Provenance;

    fn t() -> TrendCfg {
        TrendCfg::defaults()
    }

    fn point(date: &str, composite: f64, coverage: f64, phase: &str) -> TrendPoint {
        TrendPoint {
            date: date.into(),
            generated_at: format!("{}T12:00:00Z", date),
            composite,
            coverage,
            phase: phase.into(),
            stresses: BTreeMap::new(),
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
                value: 1.0,
                unit: "u".into(),
                detail: String::new(),
                provenance: Provenance {
                    source: "t".into(),
                    endpoint: "t".into(),
                    as_of: "2026-09-17".into(),
                    retrieved_at: "2026-09-17T00:00:00Z".into(),
                },
            },
            contribution: None,
        }
    }

    #[test]
    fn date_extraction_handles_timestamps_and_plain_dates() {
        assert_eq!(date_of("2026-09-17T18:22:24Z"), "2026-09-17");
        assert_eq!(date_of("2026-09-17"), "2026-09-17");
    }

    #[test]
    fn daily_series_keeps_one_point_per_date_and_prefers_the_last() {
        // Three runs on one day must not look like three days of history.
        let pts = vec![
            point("2026-09-16", 30.0, 1.0, "early"),
            point("2026-09-17", 31.0, 1.0, "early"),
            point("2026-09-17", 32.0, 1.0, "early"),
            point("2026-09-17", 33.0, 1.0, "early"),
        ];
        let s = daily_series(&pts);
        assert_eq!(s.len(), 2, "must collapse to one point per date");
        assert_eq!(s[1].composite, 33.0, "must keep the LAST run of the day");
    }

    #[test]
    fn no_history_yields_a_reason_not_a_silent_absence() {
        let current = point("2026-09-17", 32.0, 1.0, "early");
        let tr = compute(
            &[],
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        assert!(tr.delta.is_none());
        assert!(
            tr.reason.as_deref().unwrap().contains("first run"),
            "the reason must say why: {:?}",
            tr.reason
        );
    }

    #[test]
    fn delta_is_computed_against_an_eligible_baseline() {
        let archive = vec![point("2026-09-10", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        let d = tr.delta.expect("a comparable baseline exists");
        assert!((d.composite_delta - 4.0).abs() < 1e-9);
        assert_eq!(d.direction, Direction::Rising);
        assert!(
            (d.elapsed_days - 7.0).abs() < 1e-9,
            "elapsed days must be real"
        );
        assert_eq!(d.baseline_date, "2026-09-10");
    }

    #[test]
    fn a_delta_is_never_reported_without_elapsed_days() {
        let archive = vec![point("2026-09-10", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        let d = tr.delta.unwrap();
        assert!(
            d.elapsed_days > 0.0,
            "a delta without elapsed time is meaningless"
        );
    }

    #[test]
    fn baseline_is_refused_when_coverage_differs_materially() {
        // 100% vs 62%: renormalization means these are not the same scale.
        let archive = vec![point("2026-09-10", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 34.0, 0.62, "early");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        assert!(
            tr.delta.is_none(),
            "must refuse to compare runs on different scales"
        );
        let r = tr.reason.unwrap();
        assert!(
            r.contains("coverage differs by"),
            "must state the reason: {}",
            r
        );
        assert!(r.contains("not comparable"));
    }

    #[test]
    fn baseline_is_refused_when_too_recent() {
        // Same day: a re-run is not a trend.
        let archive = vec![point("2026-09-17", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        assert!(tr.delta.is_none(), "same-day re-runs must not be a trend");
        assert!(tr.reason.unwrap().contains("minimum"));
    }

    #[test]
    fn an_ineligible_recent_run_falls_back_to_an_older_eligible_one() {
        // A run recorded during an outage must not permanently blind the
        // feature, but it also must never be used itself.
        let archive = vec![
            point("2026-09-01", 29.0, 1.0, "early"),  // eligible
            point("2026-09-16", 31.0, 0.70, "early"), // too recent AND bad coverage
        ];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        let d = tr.delta.expect("an older eligible baseline exists");
        assert_eq!(d.baseline_date, "2026-09-01");
        assert!((d.composite_delta - 5.0).abs() < 1e-9);
    }

    #[test]
    fn same_day_run_is_refused_with_an_accurate_reason() {
        // Regression guard: filtering same-date entries out of the candidate
        // list made the tool claim "no previous run is recorded yet", which is
        // false when a run from earlier today exists. A refusal must not
        // misstate why it is refusing.
        let archive = vec![point("2026-09-17", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        let r = tr.reason.unwrap();
        assert!(
            !r.contains("No previous run is recorded yet"),
            "must not claim there is no history when there is: {}",
            r
        );
        assert!(r.contains("from today"), "must say why: {}", r);
    }

    #[test]
    fn direction_uses_a_dead_band_so_noise_is_not_a_signal() {
        assert_eq!(Direction::classify(0.4, 1.0), Direction::Flat);
        assert_eq!(Direction::classify(-0.4, 1.0), Direction::Flat);
        assert_eq!(Direction::classify(1.5, 1.0), Direction::Rising);
        assert_eq!(Direction::classify(-1.5, 1.0), Direction::Falling);
    }

    #[test]
    fn indicators_missing_from_either_run_are_omitted_not_zeroed() {
        let mut baseline = point("2026-09-10", 30.0, 1.0, "early");
        baseline.stresses.insert("a".into(), 40.0);
        // "b" was scored in the baseline but is a gap today.
        baseline.stresses.insert("b".into(), 10.0);

        let archive = vec![baseline];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let readings = vec![scored("a", 10.0, 50.0)];
        let tr = compute(&archive, &current, &readings, &t(), true, vec![]);
        let d = tr.delta.unwrap();
        assert_eq!(
            d.indicators.len(),
            1,
            "a gap must not be compared as if it were a number"
        );
        assert_eq!(d.indicators[0].id, "a");
        assert!((d.indicators[0].delta - 10.0).abs() < 1e-9);
    }

    #[test]
    fn unavailable_indicators_are_absent_from_the_archived_point() {
        // Zero is a legitimate stress reading, so a gap must never be written
        // as 0.0 — it must be absent.
        let gap = IndicatorReading {
            id: "credit_hy".into(),
            label: "c".into(),
            weight: 12.0,
            rationale: String::new(),
            reading: Reading::Unavailable {
                reason: "down".into(),
            },
            contribution: None,
        };
        let p = point_from(
            &[scored("a", 10.0, 42.0), gap],
            32.0,
            1.0,
            "early",
            "2026-09-17T00:00:00Z",
        );
        assert!(p.stresses.contains_key("a"));
        assert!(
            !p.stresses.contains_key("credit_hy"),
            "a gap must be absent, never archived as 0.0"
        );
    }

    #[test]
    fn phase_change_is_flagged() {
        let archive = vec![point("2026-09-10", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 40.0, 1.0, "mid");
        let tr = compute(
            &archive,
            &current,
            &[scored("a", 10.0, 50.0)],
            &t(),
            true,
            vec![],
        );
        let d = tr.delta.unwrap();
        assert!(d.phase_changed);
        assert_eq!(d.phase_then, "early");
        assert_eq!(d.phase_now, "mid");
    }

    #[test]
    fn malformed_archive_lines_are_reported_not_swallowed() {
        let dir = std::env::temp_dir().join(format!("bw-history-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let good = serde_json::to_string(&point("2026-09-10", 30.0, 1.0, "early")).unwrap();
        std::fs::write(
            archive_path(&dir),
            format!("{}\n{{ this is not json }}\n", good),
        )
        .unwrap();

        let (points, warnings) = load(&dir);
        assert_eq!(points.len(), 1, "the good line must survive");
        assert_eq!(warnings.len(), 1, "the bad line must be reported");
        assert!(
            warnings[0].contains("line 2"),
            "the warning must locate it: {}",
            warnings[0]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn archive_round_trips_and_is_append_only() {
        let dir = std::env::temp_dir().join(format!("bw-history-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        append(&dir, &point("2026-09-10", 30.0, 1.0, "early")).unwrap();
        append(&dir, &point("2026-09-11", 31.0, 1.0, "early")).unwrap();
        append(&dir, &point("2026-09-11", 32.0, 1.0, "early")).unwrap();

        let (points, warnings) = load(&dir);
        assert!(warnings.is_empty());
        assert_eq!(points.len(), 3, "every run is retained in the archive");
        assert_eq!(
            daily_series(&points).len(),
            2,
            "but only one point per date is displayed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn trend_computation_is_deterministic() {
        let archive = vec![point("2026-09-10", 30.0, 1.0, "early")];
        let current = point("2026-09-17", 34.0, 1.0, "early");
        let mk = || {
            compute(
                &archive,
                &current,
                &[scored("a", 10.0, 50.0)],
                &t(),
                true,
                vec![],
            )
        };
        assert_eq!(mk(), mk());
    }

    #[test]
    fn days_between_is_signed_and_handles_unparseable_dates() {
        assert!((days_between("2026-09-10", "2026-09-17").unwrap() - 7.0).abs() < 1e-9);
        assert!((days_between("2026-09-17", "2026-09-10").unwrap() + 7.0).abs() < 1e-9);
        assert!(days_between("not-a-date", "2026-09-17").is_none());
    }
}
