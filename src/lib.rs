//! bubble-watch — track and assess whether the AI capex/equity boom is a bubble.
//!
//! Design rule for the whole crate: **every number either carries provenance or
//! is explicitly reported as unavailable.** There is no imputation, no default
//! midpoint, and no silent fallback.

pub mod config;
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
