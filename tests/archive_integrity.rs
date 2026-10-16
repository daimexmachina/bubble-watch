//! The archive is the MEASUREMENT history, so nothing may write to it that is not a measurement.
//!
//! These are the two rules that were learned the hard way on 2026-10-16, when regenerating a report to
//! LOOK at it silently polluted the series.

use std::path::PathBuf;

fn cfg() -> bubble_watch::config::Config {
    bubble_watch::config::Config::load(std::path::Path::new("config/indicators.toml"))
        .expect("shipped config must load")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bw-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn rows_in(dir: &std::path::Path) -> usize {
    std::fs::read_to_string(dir.join("runs.jsonl"))
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

#[test]
fn an_offline_run_can_not_write_a_measurement_into_the_archive() {
    // An offline run reads the local cache; it does not consult the world. Recording it files a
    // MEASUREMENT THAT WAS NEVER TAKEN, under today's date, where it cannot be told apart from the real
    // daily run. Measured 2026-10-16: the archive held FOURTEEN rows for that day against one for every
    // other day, several of them duplicates from offline diagnostic runs with identical composites.
    //
    // The caller here asks to record (record = true). The refusal must hold anyway, because the point is
    // that no command can forget it.
    let dir = scratch("offline-record");
    let (_r, _obs) = bubble_watch::pipeline_with_history(&cfg(), true, None, &dir, true);
    let n = rows_in(&dir);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(
        n, 0,
        "an offline run must not append a measurement row; found {n}"
    );
}

#[test]
fn the_refusal_to_record_is_not_silent() {
    // A refusal that nobody can hear is its own defect: an operator who runs `report` expecting a row and
    // gets none would reasonably conclude the run succeeded. The report must SAY it did not record, and
    // why.
    let dir = scratch("offline-record-loud");
    let (r, _obs) = bubble_watch::pipeline_with_history(&cfg(), true, None, &dir, true);
    let n = rows_in(&dir);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(n, 0, "still must not record while offline");
    assert!(
        r.trend.warnings.iter().any(|w| w.contains("NOT RECORDED")),
        "the run must say it did not record: {:?}",
        r.trend.warnings
    );
}
