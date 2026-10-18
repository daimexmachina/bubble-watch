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
    // AND THE REPORT MUST NOT SAY THE OPPOSITE ELSEWHERE. `trend.recorded` is rendered into the JSON
    // and into the archived page, so a refusal that leaves it true ships a machine-readable claim that
    // the run WAS recorded, beside a human-readable warning that it was not. Measured 2026-10-17
    // (t_d8439440): `build_with_history` was handed the REQUEST (`record`) rather than the OUTCOME, so
    // this field was true for every refused run and `require_recorded` in main.rs — the backstop the
    // unattended path depends on — returned early and exited 0.
    assert!(
        !r.trend.recorded,
        "a refused record request must not report `recorded: true`; the field is the OUTCOME, not the \
         request"
    );
}

/// A run that ASKED to record and did not must exit 5, through the real CLI.
///
/// README.md, SPEC.md §7 and the `scripts/daily-refresh.sh` header all document exit `5` as "asked to
/// record a measurement and did not (an offline run, or a binary that cannot account for the source
/// tree it sits in)". Until 2026-10-17 that promise was INERT: `require_recorded` (src/main.rs) read
/// `trend.recorded`, `pipeline_with_history` passed the REQUEST into that field, and the backstop
/// therefore returned early — `--offline report` and `--offline site` printed NOT RECORDED, appended
/// nothing, and exited 0. Under systemd that is a unit reporting SUCCESS with no measurement taken,
/// which is the silence-that-looks-like-success the archive rules exist to prevent.
///
/// This spawns the built binary rather than calling `require_recorded` directly, because what was
/// broken is the EXIT CODE a scheduler sees. `history::decide_record` — the pure decision — had its own
/// tests and passed throughout the defect, which is exactly why this test has to go through the CLI.
///
/// BOTH COMMANDS, because they are two independent call sites of the same backstop: `report` is what
/// the reviewer measured, and `site` is what the 13:32 timer runs and the thing that RECORDS.
#[test]
fn a_refused_record_request_exits_5() {
    let dir = scratch("refused-record-exit");
    let cache = dir.join("cache");
    let out = dir.join("out");
    std::fs::create_dir_all(&cache).unwrap();
    let exe = env!("CARGO_BIN_EXE_bubble-watch");

    // An EMPTY cache directory, so nothing here depends on this machine's cached bodies: the offline
    // run retrieves nothing, the record rules refuse it, and the exit code is the only thing left to
    // observe. (`--history-dir` points into the scratch directory, never at data/history.)
    let runs = [
        (
            "report",
            vec![
                "report".to_string(),
                "--out".into(),
                out.display().to_string(),
                "--name".into(),
                "refused-probe".into(),
                "--history-dir".into(),
                dir.join("history-report").display().to_string(),
            ],
        ),
        (
            "site",
            vec![
                "site".to_string(),
                "--out".into(),
                out.join("site").display().to_string(),
                "--history-dir".into(),
                dir.join("history-site").display().to_string(),
            ],
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (name, args) in runs {
        let o = std::process::Command::new(exe)
            .args(["--offline", "--cache-dir"])
            .arg(&cache)
            .args(&args)
            .output()
            .expect("the binary cargo just built must be runnable");
        let stdout = String::from_utf8_lossy(&o.stdout);
        let stderr = String::from_utf8_lossy(&o.stderr);
        let history = args
            .iter()
            .position(|a| a == "--history-dir")
            .map(|i| PathBuf::from(&args[i + 1]))
            .expect("every run above passes --history-dir");
        if o.status.code() != Some(5) {
            failures.push(format!(
                "`{name}` asked to record, did not, and exited {:?} instead of 5",
                o.status.code()
            ));
        }
        if !stdout.contains("NOT RECORDED") {
            failures.push(format!("`{name}` did not print the refusal on stdout"));
        }
        if rows_in(&history) != 0 {
            failures.push(format!("`{name}` appended a row to the archive anyway"));
        }
        if !stderr.is_empty() && !stdout.contains("NOT RECORDED") {
            failures.push(format!("`{name}` stderr: {stderr}"));
        }
    }

    // The JSON is the machine-readable half of the same claim, and it is what the site publishes.
    let json = std::fs::read_to_string(out.join("refused-probe.json")).unwrap_or_default();
    if !json.contains("\"recorded\": false") {
        failures.push(format!(
            "the refused run's JSON must not claim it was recorded; it says: {:?}",
            json.lines()
                .find(|l| l.contains("recorded"))
                .unwrap_or("<no recorded field>")
        ));
    }

    std::fs::remove_dir_all(&dir).ok();
    assert!(
        failures.is_empty(),
        "a refused record request must fail loudly and non-zero:\n  {}",
        failures.join("\n  ")
    );
}
