//! bubble-watch — track and assess whether the AI capex/equity boom is a bubble.
//!
//! Design rule for the whole crate: **every number either carries provenance or
//! is explicitly reported as unavailable.** There is no imputation, no default
//! midpoint, and no silent fallback.

pub mod config;
pub mod exposure;
pub mod gsadf;
pub mod history;
pub mod http;
pub mod indicators;
pub mod model;
pub mod phase;
pub mod report;
pub mod score;
pub mod sources;

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

    let mut report =
        report::build_with_history(readings, &obs, cfg, &generated_at, &archive, record);

    // Merge load warnings with the trend the report computed for itself.
    report.trend.warnings.append(&mut warnings);

    if record {
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
