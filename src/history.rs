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
    methodology_version: &str,
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
        methodology_version: methodology_version.to_string(),
        stresses,
    }
}

/// Methodology version of an archived point, treating a missing field as "1.0"
/// so an archive written before the field existed is still classifiable rather
/// than silently unusable.
pub fn methodology_of(p: &TrendPoint) -> &str {
    if p.methodology_version.is_empty() {
        "1.0"
    } else {
        &p.methodology_version
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
///
/// NOTE: this is a CALENDAR-DATE difference and must not be used to decide whether two
/// runs are far enough apart to compare. Two runs either side of midnight differ by one
/// calendar day while being minutes apart. Use `elapsed_days` for that.
pub fn days_between(from: &str, to: &str) -> Option<f64> {
    let f = parse_date(from)?;
    let t = parse_date(to)?;
    Some((t - f).num_seconds() as f64 / 86_400.0)
}

fn parse_date(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn parse_ts(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

/// True elapsed time between two runs, in fractional days.
///
/// WHY THIS EXISTS RATHER THAN A DATE DIFFERENCE. `min_gap_days` is documented as guarding
/// against "a re-run minutes later being presented as a trend". A calendar-date comparison
/// defeats that guard at exactly the moment it matters: two runs either side of UTC midnight
/// differ by one calendar day while being minutes apart. That was not hypothetical — this
/// host's archive contained two runs 17 minutes apart, on 2026-09-19 and 2026-09-20, which
/// the date-based rule accepted as a "1 day" baseline and reported a direction of travel
/// from. A guard that fails at midnight for a cron job that runs near midnight is not a
/// guard.
///
/// Prefers `generated_at` because it carries the real instant.
///
/// FALLBACK IS DELIBERATELY CONSERVATIVE. If either timestamp is unparseable (an archive
/// written before the field was populated, or a hand-edited row), the smallest elapsed time
/// CONSISTENT with the two calendar dates is used: `date_gap - 1` days, floored at zero.
/// Two timestamps on adjacent dates can be one second apart, so a one-calendar-day
/// difference credits ZERO days. Crediting a gap that cannot be demonstrated would be the
/// same mistake as scoring a missing indicator, and the refusal is reported with its reason.
fn elapsed_days(candidate: &TrendPoint, current: &TrendPoint) -> Option<f64> {
    if let (Some(a), Some(b)) = (
        parse_ts(&candidate.generated_at),
        parse_ts(&current.generated_at),
    ) {
        return Some((b - a).num_seconds() as f64 / 86_400.0);
    }
    let d = days_between(&candidate.date, &current.date)?;
    Some((d - 1.0).max(0.0))
}

/// Render an elapsed-days value at a scale a reader can act on.
///
/// "0.0 days" tells a reader nothing; "17 minutes" tells them the comparison was refused
/// because the two runs were minutes apart. Sub-day values are rendered in hours or minutes
/// precisely because those are the cases where the smallness IS the point.
fn fmt_elapsed(d: f64) -> String {
    let ad = d.abs();
    if ad >= 1.0 {
        return format!("{} day(s)", fmt_days(d));
    }
    let mins = d * 1_440.0;
    if mins.abs() >= 60.0 {
        format!("{:.1} hour(s)", d * 24.0)
    } else {
        format!("{:.0} minute(s)", mins)
    }
}

/// Is this archived point usable as a baseline for the current run?
///
/// Both conditions are refusals, not preferences:
///   * the elapsed gap must reach `min_gap_days`, or a re-run minutes later
///     would be presented as a trend;
///   * weighted coverage must be within `coverage_tolerance_pp` of the current
///     run, or the difference partly reflects which sources answered rather than
///     a change in the market.
///
/// Each rejection carries its OWN rationale. It used to be that the report appended one
/// fixed explanation ("a change computed across runs of unequal coverage would mix a real
/// market move with the effect of which sources happened to answer") to every refusal
/// regardless of cause. That sentence is only true for the coverage case. Real archives
/// contained runs rejected purely for a methodology change at identical coverage, so the
/// report was explaining a cause that had not occurred — the same class of error as
/// inventing the number. State the reason and its rationale together, locally, so they
/// cannot drift apart.
pub fn is_eligible(
    candidate: &TrendPoint,
    current: &TrendPoint,
    t: &TrendCfg,
) -> Result<f64, String> {
    // A run from the same day is never its own baseline, regardless of how
    // `min_gap_days` is configured: comparing today with today would report
    // 0.0 as if it were a measurement of market change.
    if candidate.date == current.date {
        return Err(format!(
            "the most recent archive entry is from today, below the {} day minimum gap — a \
             comparison against today would report 0.0 as though it measured market change, \
             and the archive permits only one point per date",
            fmt_days(t.min_gap_days)
        ));
    }
    // Real elapsed time, from timestamps. NOT a calendar-date difference — see
    // `elapsed_days` for why the distinction is load-bearing.
    let gap = elapsed_days(candidate, current)
        .ok_or_else(|| format!("baseline date '{}' is unparseable", candidate.date))?;
    if gap < t.min_gap_days {
        return Err(format!(
            "the most recent baseline is {} old, below the {} day minimum — the configured \
             minimum exists so that a re-run minutes or hours later cannot be presented as a \
             trend, and the elapsed period is what makes a change interpretable. Measured from \
             the recorded timestamps ({} and {}), not from the calendar dates, because two \
             runs either side of midnight differ by a calendar day while being minutes apart",
            fmt_elapsed(gap),
            fmt_days(t.min_gap_days),
            candidate.generated_at,
            current.generated_at
        ));
    }
    // Methodology must match. Redefining an indicator changes what the composite
    // measures, so a difference across methodology changes is a difference in the
    // model, not in the market. This is a REFUSAL with a stated reason, not a
    // silent fallback: the alternative would be to report a market move that is
    // really an accounting change.
    let cm = methodology_of(candidate);
    if cm != current.methodology_version {
        return Err(format!(
            "baseline was computed under methodology {} and this run under {} — the composite \
             means something different across that change, so the two are not comparable; the \
             difference would be partly a MODEL change rather than a market move",
            cm, current.methodology_version
        ));
    }

    let diff_pp = (candidate.coverage - current.coverage).abs() * 100.0;
    if diff_pp > t.coverage_tolerance_pp {
        return Err(format!(
            "coverage differs by {:.1}pp (baseline {:.0}% vs current {:.0}%), above the {:.1}pp \
             tolerance — the difference would partly reflect WHICH SOURCES answered rather than \
             a change in the market, so the two runs are not on the same scale",
            diff_pp,
            candidate.coverage * 100.0,
            current.coverage * 100.0,
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
        match is_eligible(c, current, t) {
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

    // Deduplicate the reasons: several archived entries can be rejected for the
    // identical cause (three runs recorded on one day all fail the gap test), and
    // repeating the same sentence once per entry is noise, not information.
    let mut unique: Vec<String> = Vec::new();
    for r in rejections {
        if !unique.contains(&r) {
            unique.push(r);
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
            unique.join("; ")
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

    const METHOD: &str = "1.1";

    fn point(date: &str, composite: f64, coverage: f64, phase: &str) -> TrendPoint {
        point_v(date, composite, coverage, phase, METHOD)
    }

    /// A point whose real timestamp is NOT the date's noon, so midnight-straddling
    /// cases can be expressed exactly.
    fn point_ts(date: &str, ts: &str, composite: f64) -> TrendPoint {
        let mut p = point(date, composite, 1.0, "early");
        p.generated_at = ts.into();
        p
    }

    fn point_v(date: &str, composite: f64, coverage: f64, phase: &str, method: &str) -> TrendPoint {
        TrendPoint {
            date: date.into(),
            generated_at: format!("{}T12:00:00Z", date),
            composite,
            coverage,
            phase: phase.into(),
            methodology_version: method.into(),
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
    fn a_run_minutes_apart_is_never_a_baseline_even_across_midnight() {
        // THE DEFECT THIS GUARDS. The gap test used a CALENDAR-DATE difference, so two
        // runs either side of UTC midnight differed by "1 day" while being minutes apart.
        // This host's archive really contained such a pair: 2026-09-19T23:47:36Z and
        // 2026-09-20T00:04:49Z, 17 minutes apart, and the tool reported a direction of
        // travel from it. `min_gap_days` is documented as existing so that "a re-run
        // minutes later" cannot be presented as a trend — the guard was failing at exactly
        // the case it names, and only near midnight, which is where a cron job can land.
        let baseline = point_ts("2026-09-19", "2026-09-19T23:47:36Z", 30.0);
        let current = point_ts("2026-09-20", "2026-09-20T00:04:49Z", 30.0);

        let why = is_eligible(&baseline, &current, &t())
            .expect_err("17 minutes must not qualify as a baseline");
        assert!(
            why.contains("minute"),
            "the refusal must state the real elapsed time, not '1 day': {}",
            why
        );
        assert!(
            !why.contains("1 day(s) old"),
            "must not claim a day elapsed when 17 minutes did: {}",
            why
        );
        // And it must say the measurement came from timestamps, since that is the fix.
        assert!(why.contains("timestamps"), "must state the basis: {}", why);
    }

    #[test]
    fn a_genuine_daily_gap_is_accepted() {
        // The complement: the fix must not become a guard that refuses everything. A real
        // daily run ~24h later is a legitimate baseline.
        let baseline = point_ts("2026-09-18", "2026-09-18T20:34:16Z", 30.0);
        let current = point_ts("2026-09-19", "2026-09-19T20:36:15Z", 34.0);
        let gap = is_eligible(&baseline, &current, &t()).expect("a ~24h gap is comparable");
        assert!(
            (gap - 1.001).abs() < 0.01,
            "the reported gap must be the REAL elapsed time: {}",
            gap
        );
    }

    #[test]
    fn elapsed_time_falls_back_conservatively_when_timestamps_are_unusable() {
        // An archive row with an unparseable timestamp must not be credited with a gap it
        // cannot demonstrate. Two calendar dates one day apart are consistent with a gap of
        // anything from one second to ~48 hours, so the floor is zero days.
        let mut baseline = point("2026-09-18", 30.0, 1.0, "early");
        baseline.generated_at = "not-a-timestamp".into();
        let mut current = point("2026-09-19", 34.0, 1.0, "early");
        current.generated_at = "also-not-a-timestamp".into();

        let gap = elapsed_days(&baseline, &current).expect("dates are still parseable");
        assert!(
            gap.abs() < 1e-9,
            "a one-date difference cannot demonstrate a full day, got {}",
            gap
        );
        // So the candidate is refused, with a reason, rather than silently credited.
        assert!(is_eligible(&baseline, &current, &t()).is_err());
    }

    #[test]
    fn sub_day_durations_render_at_a_scale_a_reader_can_use() {
        assert_eq!(fmt_elapsed(0.0119), "17 minute(s)");
        assert_eq!(fmt_elapsed(0.5), "12.0 hour(s)");
        assert_eq!(fmt_elapsed(1.0), "1 day(s)");
        assert_eq!(fmt_elapsed(7.0), "7 day(s)");
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
    fn a_refusal_never_borrows_a_cause_that_did_not_occur() {
        // The report used to append one fixed rationale about unequal COVERAGE to every
        // refusal. Real archives contained runs rejected purely for a METHODOLOGY change
        // at coverage 1.0 on both sides, so the explanation named a cause that had not
        // happened. Each rejection now carries its own rationale, and this asserts the
        // coverage rationale appears ONLY when coverage is actually the problem.
        let t = TrendCfg {
            // 1.0 here on purpose: this test wants a strict threshold so its
            // candidates are separated by cause, not by duration.
            min_gap_days: 1.0,
            coverage_tolerance_pp: 2.0,
            ..t()
        };
        // The "current" run these candidates are tested against: methodology 2.0 so the
        // methodology branch is reached only when a candidate deliberately differs, and
        // coverage 1.0 so the coverage branch is reached only when a candidate differs.
        let mut cur = point("2026-09-19", 30.0, 1.0, "early");
        cur.methodology_version = "2.0".into();

        // Identical coverage, different methodology -> must NOT mention coverage.
        let mut c = point("2026-09-01", 30.0, 1.0, "early");
        c.methodology_version = "1.9".into();
        let why = is_eligible(&c, &cur, &t).unwrap_err();
        assert!(
            !why.to_lowercase().contains("coverage"),
            "a methodology refusal must not blame coverage: {}",
            why
        );
        assert!(
            why.contains("methodology"),
            "and it must name the real cause: {}",
            why
        );

        // Same-day -> must say today, not coverage.
        let same = point(&cur.date.clone(), 30.0, 1.0, "early");
        let why = is_eligible(&same, &cur, &t).unwrap_err();
        assert!(
            !why.to_lowercase().contains("coverage"),
            "a same-day refusal must not blame coverage: {}",
            why
        );
        assert!(why.contains("today"), "must say why: {}", why);

        // Insufficient elapsed gap -> must not blame coverage.
        let near = point("2026-09-19", 30.0, 1.0, "early");
        let why = is_eligible(&near, &cur, &t).unwrap_err();
        assert!(
            !why.to_lowercase().contains("coverage"),
            "a gap refusal must not blame coverage: {}",
            why
        );

        // Genuine coverage mismatch -> NOW the coverage rationale is correct and required.
        // Methodology must MATCH here, or the rejection is correctly about methodology
        // first and the coverage branch is never reached (which is what an earlier
        // version of this test tripped over).
        let mut c2 = point("2026-09-01", 30.0, 0.55, "early");
        c2.methodology_version = "2.0".into();
        let why = is_eligible(&c2, &cur, &t).unwrap_err();
        assert!(
            why.to_lowercase().contains("coverage"),
            "a coverage refusal must name coverage: {}",
            why
        );
        assert!(
            why.contains("SOURCES"),
            "and must explain that it is about which sources answered: {}",
            why
        );
    }

    #[test]
    fn repeated_rejection_reasons_are_not_repeated_in_the_message() {
        // Three runs on one day all fail the gap test for the SAME reason; the
        // refusal must say it once. Repeating it per entry is noise.
        let archive = vec![
            point("2026-09-17", 30.0, 1.0, "early"),
            point("2026-09-17", 31.0, 1.0, "early"),
            point("2026-09-17", 32.0, 1.0, "early"),
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
        let r = tr.reason.unwrap();
        // Derive the expected substring from the ACTUAL configured threshold rather than
        // hardcoding it: the literal went stale when min_gap_days changed from 1 to 0.9,
        // and a test that breaks on a config change it does not care about trains people
        // to update literals instead of reading failures.
        let gap_label = format!("below the {} day minimum gap", fmt_days(t().min_gap_days));
        assert_eq!(
            r.matches(&gap_label).count(),
            1,
            "the same rejection must be stated once (looking for {:?}): {}",
            gap_label,
            r
        );
        assert!(
            !r.contains(".."),
            "must not emit a doubled full stop: {}",
            r
        );
    }

    #[test]
    fn baseline_is_refused_across_a_methodology_change() {
        // Redefining an indicator changes what the composite MEANS. Comparing
        // across that change would report a model change as a market move.
        let archive = vec![point_v("2026-09-10", 30.0, 1.0, "early", "1.0")];
        let current = point_v("2026-09-17", 34.0, 1.0, "early", "1.1");
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
            "must refuse to compare across a methodology change"
        );
        let r = tr.reason.unwrap();
        assert!(r.contains("methodology 1.0"), "must name both: {}", r);
        assert!(r.contains("1.1"));
        assert!(r.contains("not comparable"));
    }

    #[test]
    fn a_missing_methodology_field_is_treated_as_the_original_schema() {
        // Archives written before the field existed must still load, defaulting
        // to 1.0 rather than being silently unusable.
        let json = r#"{"date":"2026-01-01","generated_at":"2026-01-01T00:00:00Z",
                       "composite":20.0,"coverage":1.0,"phase":"early","stresses":{}}"#;
        let p: TrendPoint = serde_json::from_str(json).unwrap();
        assert_eq!(methodology_of(&p), "1.0");
        assert!(p.methodology_version.is_empty());
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
            METHOD,
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
