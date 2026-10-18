//! bubble-watch — track and assess whether the AI capex/equity boom is a bubble.
//!
//! Design rule for the whole crate: **every number either carries provenance or
//! is explicitly reported as unavailable.** There is no imputation, no default
//! midpoint, and no silent fallback.

pub mod backtest;
/// A content fingerprint of the source tree, computed the SAME WAY at build time and at run
/// time — the build script bakes one into the binary and `build_id` recomputes it from disk.
///
/// The implementation file uses `//` comments rather than `//!` because it is spliced into
/// `build.rs` by `include!`, which cannot carry an inner attribute; see the top of
/// `src/build_fingerprint.rs` for the full reasoning, including why the fingerprint is a content
/// hash rather than an mtime comparison.
pub mod build_fingerprint;
pub mod build_id;
pub mod circular_rubric;
pub mod config;
pub mod drift;
pub mod exposure;
pub mod falsifiers;
pub mod frontier;
pub mod gsadf;
pub mod history;
pub mod http;
pub mod indicators;
pub mod model;
pub mod phase;
pub mod report;
pub mod score;
pub mod sources;
pub mod subjective;

/// Current UTC time as ISO-8601, or a clear marker if the clock is unusable.
pub fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Current UTC date as YYYY-MM-DD.
pub fn now_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Run the full pipeline: fetch, evaluate, assemble. The only IO is `fetch`.
///
/// No history is consulted and no run is recorded: direction of travel is
/// reported as absent, with a reason. Use `pipeline_with_history` for the trend.
pub fn pipeline(
    cfg: &config::Config,
    offline: bool,
    cache_dir: Option<std::path::PathBuf>,
) -> (model::Report, model::Observations) {
    let fetcher = http::Fetcher::new(offline, cache_dir);
    let obs = sources::fetch_all(&fetcher, offline);
    let ctx = indicators::Ctx { obs: &obs, cfg };
    let readings = indicators::evaluate_all(&ctx);
    let report = report::build(readings, &obs, cfg, &now_iso8601());
    (report, obs)
}

/// Run the pipeline and compute direction of travel against the run archive.
///
/// Ordering matters and is deliberate:
///
/// 1. load the existing archive (malformed lines become warnings, not failures);
/// 2. compute the report against THAT pre-existing archive, so today's run is
///    never used as its own baseline — `history::compute` additionally filters
///    same-date entries, so a re-run cannot compare against itself;
/// 3. append the new run, so the history is durable even if writing the report
///    later fails.
///
/// A failure to write the archive is downgraded to a warning on the report
/// rather than aborting the run: losing history is bad, but losing the report is
/// worse.
pub fn pipeline_with_history(
    cfg: &config::Config,
    offline: bool,
    cache_dir: Option<std::path::PathBuf>,
    history_dir: &std::path::Path,
    record: bool,
) -> (model::Report, model::Observations) {
    let fetcher = http::Fetcher::new(offline, cache_dir);
    let obs = sources::fetch_all(&fetcher, offline);
    let ctx = indicators::Ctx { obs: &obs, cfg };
    let readings = indicators::evaluate_all(&ctx);

    let (archive, mut warnings) = history::load(history_dir);
    let generated_at = now_iso8601();

    // `recorded` IS THE OUTCOME, NOT THE REQUEST, AND THAT IS WHY `false` IS PASSED HERE.
    //
    // It is set true below, and only by a successful `append`. Passing `record` (the request) made
    // `trend.recorded` true for a run the rules had just REFUSED, which broke the same field in three
    // places at once: the JSON claimed `"recorded": true` next to a NOT RECORDED warning, and
    // `require_recorded` in main.rs — whose entire job is to fail the unattended path — read the
    // request, returned early, so `report` and `site` exited 0 having appended nothing. Measured
    // 2026-10-17 (t_d8439440): `--offline report` printed NOT RECORDED, left the archive byte-identical,
    // and wrote `"recorded": true` into the probe JSON. Under systemd that is a unit reporting SUCCESS
    // with no measurement taken, which is the silence-that-looks-like-success the archive rules exist
    // to prevent.
    //
    // The alternative fix — leave the request here and have `require_recorded` scan for a NOT RECORDED
    // warning — was rejected. It would leave this field meaning the REQUEST in the JSON while the
    // terminal meaning the OUTCOME, and it would not catch a failed `append`, whose warning is
    // "this run was NOT recorded to the archive" and is not the string `decide_record` emits.
    let mut report =
        report::build_with_history(readings, &obs, cfg, &generated_at, &archive, false, true);

    // Merge load warnings with the trend the report computed for itself.
    report.trend.warnings.append(&mut warnings);

    // RECORDING IS GOVERNED BY `history::decide_record`, WHICH IS A PURE FUNCTION AND LIVES NEXT TO
    // `append` FOR THAT REASON. Two rules, both data-integrity rules rather than conveniences:
    //
    //   1. AN OFFLINE RUN MAY NOT RECORD. It reads whatever is in the local cache; it does not consult
    //      the world, so appending it files a MEASUREMENT that was never taken — under today's date,
    //      where it is indistinguishable from the real daily run. Measured 2026-10-16: the archive held
    //      FOURTEEN rows for that day against one for every other day, several of them duplicates from
    //      offline diagnostic runs with identical composites, and nothing said which row was real.
    //
    //   2. A BINARY THAT CANNOT ACCOUNT FOR ITS OWN SOURCE MAY NOT RECORD. `scripts/falsify.sh`
    //      restores the mutated SOURCE and proves the restore, but its `cargo test` also builds the
    //      BINARY target from the MUTATED source, and it used to leave that binary in place with a clean
    //      tree. `scripts/daily-refresh.sh` rebuilt only when the binary was missing, so the next timer
    //      run would have executed the mutant and RECORDED it into the archive. Measured 2026-10-17: a
    //      binary left by a falsification printed "plus 5 things it cannot measure at all" where the
    //      committed source prints 2. The binary now carries the fingerprint of the tree it was built
    //      from (`build.rs`) and refuses to record when that fingerprint is not the tree it is running
    //      beside. See `crate::build_id`.
    //
    // The decision is computed here rather than in each command, so no command can forget it, and every
    // refusal is pushed onto the report so it reaches the HTML, the JSON and the terminal.
    let artifact = build_id::check();
    let decision = history::decide_record(record, offline, &artifact);
    for r in &decision.refusals {
        report.trend.warnings.push(r.clone());
    }
    if decision.record {
        let point = history::point_from(
            &report.indicators,
            report.composite,
            report.coverage,
            &report.phase,
            &generated_at,
            &cfg.meta.schema_version,
        );
        // Persist the full report page for this date BEFORE appending the
        // archive entry, so the archive never contains a date whose page is
        // missing. If the page cannot be written, the run is still recorded
        // (the numbers matter more than the page) and the failure is surfaced.
        let date = history::date_of(&generated_at);
        match history::save_report_html(history_dir, &date, &report::html::render(&report)) {
            Ok(_) => {}
            Err(e) => report
                .trend
                .warnings
                .push(format!("archived report page not written: {}", e)),
        }
        match history::append(history_dir, &point) {
            Ok(()) => report.trend.recorded = true,
            Err(e) => report
                .trend
                .warnings
                .push(format!("this run was NOT recorded to the archive: {}", e)),
        }
    }

    (report, obs)
}
