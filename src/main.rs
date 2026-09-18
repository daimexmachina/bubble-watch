//! CLI entry point.

use bubble_watch::{config::Config, model::Reading, report};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "bubble-watch",
    version,
    about = "Assess whether the AI capex/equity boom is a bubble, with published weights and auditable math.",
    long_about = "Reports a composite bubble-stress score derived from documented indicators. \
Every weight, anchor and threshold lives in config/indicators.toml. Indicators whose sources \
are unavailable are reported as gaps and contribute nothing — they are never imputed.\n\n\
This tool does not predict when a market will reverse, and it is not investment advice."
)]
struct Cli {
    /// Path to the indicator configuration.
    #[arg(long, global = true, default_value = "config/indicators.toml")]
    config: PathBuf,

    /// Use only cached responses; never touch the network.
    #[arg(long, global = true)]
    offline: bool,

    /// Directory for the HTTP response cache.
    #[arg(long, global = true, default_value = "data/cache")]
    cache_dir: PathBuf,

    /// Print the HTTP request log to stderr.
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Fetch all sources and report source health.
    Fetch {
        /// Write the raw retrieved observations to this path as JSON. Used to
        /// capture a real, committed fixture so the offline test suite and the
        /// golden report run against genuine market data rather than invented
        /// numbers.
        #[arg(long)]
        dump_observations: Option<PathBuf>,
    },
    /// Compute and print the composite score.
    Score {
        /// Emit compact JSON instead of a human summary.
        #[arg(long)]
        json: bool,
    },
    /// Write a JSON report and a self-contained HTML dashboard.
    Report {
        /// Output directory.
        #[arg(long, default_value = ".")]
        out: PathBuf,
        /// Base filename (without extension).
        #[arg(long, default_value = "bubble-report")]
        name: String,
        /// Directory holding the run archive used for direction of travel.
        #[arg(long, default_value = "data/history")]
        history_dir: PathBuf,
        /// Do not append this run to the archive.
        #[arg(long)]
        no_record: bool,
    },
    /// Show exactly how the composite was built, indicator by indicator.
    Explain,
    /// Probe each source and print its health.
    Sources,
    /// Show direction of travel: the recorded run history and the delta against
    /// the most recent comparable baseline.
    Trend {
        /// Directory holding the run archive.
        #[arg(long, default_value = "data/history")]
        history_dir: PathBuf,
    },
    /// Generate the self-contained site (dashboard + per-day reports + index)
    /// into a directory that can be served with any static file server.
    Site {
        /// Output directory. Defaults to the configured site directory.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Directory holding the run archive.
        #[arg(long, default_value = "data/history")]
        history_dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        eprintln!("error: {}", e);
        std::process::exit(2);
    }
}

fn load_config(cli: &Cli) -> Result<Config, String> {
    Config::load(&cli.config).map_err(|e| e.to_string())
}

fn run(cli: &Cli) -> Result<(), String> {
    let cfg = load_config(cli)?;
    let cache = if cli.cache_dir.as_os_str().is_empty() {
        None
    } else {
        Some(cli.cache_dir.clone())
    };

    match &cli.cmd {
        Cmd::Sources => {
            let f = bubble_watch::http::Fetcher::new(cli.offline, cache.clone());
            let obs = bubble_watch::sources::fetch_all(&f, cli.offline);
            let health = bubble_watch::sources::source_health(&obs);
            println!(
                "{:<14} {:<12} {:<16} {}",
                "SOURCE", "STATUS", "COUNTS", "DETAIL"
            );
            for s in &health {
                println!(
                    "{:<14} {:<12} {:<16} {}",
                    s.name,
                    s.status,
                    format!("{} ok / {} fail", s.ok_count, s.failed_count),
                    s.detail
                );
            }
            if !obs.failures.is_empty() {
                println!("\nFailures ({}):", obs.failures.len());
                for f in &obs.failures {
                    println!("  [{}] {} :: {}", f.source, f.endpoint, f.reason);
                }
            }
            if cli.verbose {
                println!("\nRequest log:");
                for line in f.log.borrow().iter() {
                    println!("  {}", line);
                }
            }
            // Partial failure is normal and must not look like success.
            let degraded = health.iter().any(|s| s.status != "ok");
            if degraded {
                std::process::exit(4);
            }
            Ok(())
        }
        Cmd::Fetch { dump_observations } => {
            let f = bubble_watch::http::Fetcher::new(cli.offline, cache.clone());
            let obs = bubble_watch::sources::fetch_all(&f, cli.offline);
            let n_y = obs.yahoo.len();
            let n_f = obs.fred.len();
            let n_e = obs.edgar.len();
            println!(
                "fetched: yahoo={} fred={} edgar_filers={} failures={}",
                n_y,
                n_f,
                n_e,
                obs.failures.len()
            );
            println!("retrieved_at: {}", obs.retrieved_at);
            if let Some(path) = dump_observations {
                let json = serde_json::to_string_pretty(&obs)
                    .map_err(|e| format!("cannot serialise observations: {}", e))?;
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                std::fs::write(&path, json).map_err(|e| e.to_string())?;
                println!("wrote {}", path.display());
            }
            if n_y == 0 && n_e == 0 {
                eprintln!("all primary sources failed — nothing usable was retrieved");
                std::process::exit(3);
            }
            if cli.verbose {
                for line in f.log.borrow().iter() {
                    println!("  {}", line);
                }
            }
            Ok(())
        }
        Cmd::Score { json } => {
            let (r, _) = bubble_watch::pipeline(&cfg, cli.offline, cache.clone());
            if *json {
                println!("{}", report::json::to_json(&r, true));
            } else {
                print_human(&r);
            }
            finish_code(&r);
            Ok(())
        }
        Cmd::Report {
            out,
            name,
            history_dir,
            no_record,
        } => {
            let (r, _) = bubble_watch::pipeline_with_history(
                &cfg,
                cli.offline,
                cache.clone(),
                history_dir,
                !*no_record,
            );
            std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
            let json_path = out.join(format!("{}.json", name));
            let html_path = out.join(format!("{}.html", name));
            std::fs::write(&json_path, report::json::to_json(&r, true))
                .map_err(|e| e.to_string())?;
            std::fs::write(&html_path, report::html::render(&r)).map_err(|e| e.to_string())?;
            println!("wrote {}", json_path.display());
            println!("wrote {}", html_path.display());
            print_human(&r);
            finish_code(&r);
            Ok(())
        }
        Cmd::Explain => {
            let (r, obs) = bubble_watch::pipeline(&cfg, cli.offline, cache.clone());
            explain(&r, &obs);
            finish_code(&r);
            Ok(())
        }
        Cmd::Trend { history_dir } => {
            let (r, _) = bubble_watch::pipeline_with_history(
                &cfg,
                cli.offline,
                cache.clone(),
                history_dir,
                false,
            );
            print_trend(&r);
            finish_code(&r);
            Ok(())
        }
        Cmd::Site { out, history_dir } => {
            // Generate today's run and record it, then rebuild the whole site so
            // the dashboard reflects this run immediately.
            let (r, _) = bubble_watch::pipeline_with_history(
                &cfg,
                cli.offline,
                cache.clone(),
                history_dir,
                true,
            );
            let dir = out.clone().unwrap_or_else(|| PathBuf::from("site"));
            gen_site(&dir, history_dir, &r)?;
            print_human(&r);
            finish_code(&r);
            Ok(())
        }
    }
}

/// Write the servable site: a dashboard, an index, and one report per recorded
/// run plus a `latest.html` alias.
///
/// Per-day reports are only written for dates present in the archive, so the
/// site never links to a page that does not exist.
fn gen_site(
    dir: &std::path::Path,
    history_dir: &std::path::Path,
    latest: &bubble_watch::model::Report,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let (points, warnings) = bubble_watch::history::load(history_dir);
    for w in &warnings {
        eprintln!("warning: {}", w);
    }
    let series = bubble_watch::history::daily_series(&points);

    // Publish the archived per-run report pages into the site, so any past date
    // can be opened in full.
    let archived = bubble_watch::history::reports_dir(history_dir);
    let mut report_pages = std::collections::BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir(&archived) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(date) = name.strip_suffix(".html") {
                let dst = dir.join(&name);
                if std::fs::copy(e.path(), &dst).is_ok() {
                    report_pages.insert(date.to_string());
                }
            }
        }
    }

    // The current run's own report, plus aliases for convenience.
    std::fs::write(dir.join("latest.html"), report::html::render(latest))
        .map_err(|e| e.to_string())?;
    let today = bubble_watch::history::date_of(&latest.generated_at);
    std::fs::write(
        dir.join(format!("{}.html", today)),
        report::html::render(latest),
    )
    .map_err(|e| e.to_string())?;
    report_pages.insert(today.clone());

    let have_page = |d: &str| report_pages.contains(d);

    std::fs::write(
        dir.join("dashboard.html"),
        report::html::render_dashboard(&series, Some(latest), cfg_sparkline_points(), &have_page),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(dir.join("index.html"), report::html::render_index())
        .map_err(|e| e.to_string())?;

    // A plain-text copy of the series, so the history is usable without a
    // browser and diffable in git.
    let mut csv = String::from("date,composite,coverage,phase\n");
    for p in &series {
        csv.push_str(&format!(
            "{},{:.4},{:.4},{}\n",
            p.date, p.composite, p.coverage, p.phase
        ));
    }
    std::fs::write(dir.join("series.csv"), csv).map_err(|e| e.to_string())?;

    println!("wrote {}", dir.join("dashboard.html").display());
    println!("wrote {}", dir.join("index.html").display());
    println!("wrote {}", dir.join("latest.html").display());
    println!("wrote {}", dir.join("series.csv").display());
    println!(
        "site contains {} recorded run(s); {} full report page(s) published",
        series.len(),
        report_pages.len()
    );
    Ok(())
}

/// Sparkline length, read from the shipped config so the site and the report
/// agree on how much history to draw.
fn cfg_sparkline_points() -> usize {
    Config::load(std::path::Path::new("config/indicators.toml"))
        .map(|c| c.trend.sparkline_points)
        .unwrap_or(30)
}

/// Direction of travel, in a form that makes the comparability rule visible
/// rather than merely implied.
fn print_trend(r: &bubble_watch::model::Report) {
    println!("DIRECTION OF TRAVEL");
    println!("{}", "=".repeat(72));
    println!("{}", wrap(&r.layman.direction_of_travel, 72));
    println!();

    for w in &r.trend.warnings {
        println!("  ! {}", w);
    }

    println!(
        "recorded runs (one per date, newest last): {}",
        r.trend.points.len()
    );
    if r.trend.points.is_empty() {
        println!("  (no runs recorded yet)");
    } else {
        println!(
            "  {:<12} {:>9} {:>9}  {}",
            "DATE", "COMPOSITE", "COVERAGE", "PHASE"
        );
        for p in &r.trend.points {
            println!(
                "  {:<12} {:>9.2} {:>8.0}%  {}",
                p.date,
                p.composite,
                p.coverage * 100.0,
                p.phase
            );
        }
    }
    println!("{}", "-".repeat(72));

    match &r.trend.delta {
        Some(d) => {
            println!(
                "vs {} ({:.0} days): composite {:+.2} -> {}  [{} phase {} {} phase]",
                d.baseline_date,
                d.elapsed_days,
                d.composite_delta,
                d.direction.as_str(),
                d.phase_then,
                if d.phase_changed { "->" } else { "=" },
                d.phase_now
            );
            println!(
                "baseline coverage {:.0}% vs current {:.0}% (within tolerance)",
                d.baseline_coverage * 100.0,
                r.coverage * 100.0
            );
            if !d.indicators.is_empty() {
                println!();
                println!(
                    "  {:<26} {:>9} {:>9} {:>8}  {}",
                    "INDICATOR", "THEN", "NOW", "CHANGE", "DIRECTION"
                );
                for i in &d.indicators {
                    println!(
                        "  {:<26} {:>9.1} {:>9.1} {:>+8.1}  {}",
                        i.id,
                        i.baseline_stress,
                        i.current_stress,
                        i.delta,
                        i.direction.as_str()
                    );
                }
            }
        }
        None => {
            println!("No delta reported.");
            if let Some(reason) = &r.trend.reason {
                println!("{}", wrap(reason, 72));
            }
        }
    }
    println!("{}", "=".repeat(72));
    println!(
        "The trend is CONTEXT only: it never enters the composite, which is derived above it."
    );
}

fn finish_code(r: &bubble_watch::model::Report) {
    if r.data_quality.available_weight == 0.0 {
        std::process::exit(3);
    }
    if r.coverage < 0.5 {
        std::process::exit(4);
    }
}

fn print_human(r: &bubble_watch::model::Report) {
    // Plain-English summary first, so a non-specialist can read the top of the
    // output and stop there if they want to.
    println!("PLAIN-ENGLISH SUMMARY");
    println!("{}", "=".repeat(72));
    println!("{}", wrap(&r.layman.what_this_is, 72));
    println!();
    println!("{}", wrap(&r.layman.the_score, 72));
    println!();
    println!("{}", wrap(&r.layman.what_is_stretched, 72));
    println!();
    println!("{}", wrap(&r.layman.what_is_calm, 72));
    println!();
    println!("{}", wrap(&r.layman.what_we_cannot_measure, 72));
    println!();
    println!("{}", wrap(&r.layman.about_timing, 72));
    println!();
    println!("{}", wrap_block(&r.layman.bottom_line, 72));
    println!();
    println!("{}", "=".repeat(72));
    println!("DETAIL (for reference)");
    println!("{}", "=".repeat(72));

    println!(
        "AI Bubble Watch — composite {:.1}/100  [{}]",
        r.composite,
        r.phase.to_uppercase()
    );
    println!(
        "coverage {:.0}%  confidence {}  generated {}",
        r.coverage * 100.0,
        r.confidence,
        r.generated_at
    );
    println!("{}", "-".repeat(72));
    println!("{}", r.phase_label);

    // Per-company exposure. CONTEXT, not part of the composite: company-level
    // analysis and a market-level score answer different questions.
    if !r.exposure.is_empty() {
        println!();
        println!("PER-COMPANY EXPOSURE (context only - never enters the composite)");
        println!(
            "{:<8}{:>11}{:>11}{:>9}{:>14}{:>11}",
            "COMPANY", "debt/CFO", "due<1y/CFO", "leases", "commitments", "RPO/rev"
        );
        for e in &r.exposure {
            let fmt = |v: Option<f64>, partial: bool| match v {
                Some(x) => format!("{:.2}", x),
                // Never render a missing value as 0.00. An absent disclosure is
                // not a small number.
                None => {
                    if partial {
                        "n/d".to_string()
                    } else {
                        "-".to_string()
                    }
                }
            };
            println!(
                "{:<8}{:>11}{:>11}{:>9}{:>14}{:>11}",
                e.ticker,
                fmt(e.debt_to_cfo, false),
                fmt(e.near_term_to_cfo, false),
                fmt(e.lease_to_cfo, false),
                fmt(e.commitments_to_cfo, e.is_partial()),
                fmt(e.rpo_to_revenue, false)
            );
        }
        println!("  'n/d' = not disclosed; a missing figure is named, never shown as 0.00.");
        println!("  Ordered most-exposed first by a simple rank key, not a probability.");
        for e in &r.exposure {
            for n in &e.notes {
                println!("    {}: {}", e.ticker, n);
            }
        }
    }

    // The second, independent method. Printed next to the composite and never
    // reconciled with it: if the two disagree, that IS the finding.
    match &r.explosiveness {
        Some(e) => {
            println!();
            println!(
                "explosiveness test (GSADF): statistic {:.2}, {} (simulated 5% critical value {:.2})",
                e.statistic, e.significance, e.critical.p95
            );
            println!(
                "  window tested: {} to {} over {} observations",
                e.window_start_date, e.window_end_date, e.observations
            );
            println!(
                "  This is a formal hypothesis test on the price series, NOT part of the composite."
            );
            println!(
                "  If it disagrees with the score above, that disagreement is the point: the two"
            );
            println!("  measure different things and neither is a forecast.");
        }
        None => {}
    }
    match (r.analog.range_months, &r.analog.band) {
        (Some(rg), Some(b)) => println!(
            "historical analog: band '{}' — {:.0}-{:.0} months to a peak in the reference analogs (NOT a probability)",
            b, rg[0], rg[1]
        ),
        _ => println!(
            "historical analog: {}",
            r.analog.band_note.as_deref().unwrap_or("not available")
        ),
    }
    println!("{}", "-".repeat(72));
    println!(
        "{:<26} {:>6} {:>7} {:>7}",
        "INDICATOR", "WEIGHT", "STRESS", "CONTRIB"
    );
    for i in &r.indicators {
        match &i.reading {
            Reading::Scored { stress, .. } => println!(
                "{:<26} {:>6.0} {:>7.1} {:>7}",
                i.id,
                i.weight,
                stress,
                i.contribution
                    .map(|c| format!("{:.2}", c))
                    .unwrap_or_else(|| "—".into())
            ),
            Reading::Unavailable { .. } => {
                println!("{:<26} {:>6.0} {:>7} {:>7}", i.id, i.weight, "GAP", "—")
            }
        }
    }
    println!("{}", "-".repeat(72));
    if !r.data_quality.unavailable.is_empty() {
        println!("GAPS ({}):", r.data_quality.unavailable.len());
        for g in &r.data_quality.unavailable {
            println!("  {} — {}", g.id, g.reason);
        }
    }
    println!("\n{}", r.headline);
    for c in &r.caveats {
        println!("  ! {}", c);
    }
    println!("\n{}", r.disclaimer);
}

fn wrap_block(text: &str, width: usize) -> String {
    let mut out = String::new();
    for (i, line) in wrap(text, width - 2).lines().enumerate() {
        if i == 0 {
            out.push_str(&format!("| {}\n", line));
        } else {
            out.push_str(&format!("  {}\n", line));
        }
    }
    out.push_str(&format!("{}", "|".to_string()));
    out
}

/// Greedy word wrap, so long plain-English sentences stay readable in a
/// terminal instead of running off the edge.
fn wrap(text: &str, width: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines.join("\n")
}

fn explain(r: &bubble_watch::model::Report, obs: &bubble_watch::model::Observations) {
    println!("HOW THE COMPOSITE WAS BUILT");
    println!("{}", "=".repeat(72));
    let cfg = Config::load(std::path::Path::new("config/indicators.toml")).ok();
    println!(
        "composite = sum(weight_i * stress_i) / sum(weight_i) over AVAILABLE indicators only\n"
    );
    for i in &r.indicators {
        println!("[{}]  {}", i.id, i.label);
        println!("  configured weight : {}", i.weight);
        println!("  rationale         : {}", i.rationale);
        match &i.reading {
            Reading::Scored {
                stress,
                value,
                unit,
                detail,
                provenance,
            } => {
                println!("  raw value         : {:.4} {}", value, unit);
                println!("  stress            : {:.2} / 100", stress);
                println!(
                    "  contribution      : {}",
                    i.contribution
                        .map(|c| format!("{:.4}", c))
                        .unwrap_or_else(|| "0 (weight 0)".into())
                );
                if let Some(c) = cfg.as_ref().and_then(|c| c.indicator(&i.id)) {
                    let a: Vec<String> = c
                        .anchors
                        .iter()
                        .map(|p| format!("({}, {})", p[0], p[1]))
                        .collect();
                    println!("  anchors           : {}", a.join(" -> "));
                }
                println!("  detail            : {}", detail);
                println!(
                    "  provenance        : {} | {} | as of {} | retrieved {}",
                    provenance.source,
                    provenance.endpoint,
                    provenance.as_of,
                    provenance.retrieved_at
                );
            }
            Reading::Unavailable { reason } => {
                println!("  UNAVAILABLE — contributes nothing to the composite");
                println!("  reason            : {}", reason);
            }
        }
        println!();
    }
    println!("{}", "=".repeat(72));
    println!(
        "coverage {:.0}% ({} of {} weight available)",
        r.coverage * 100.0,
        r.data_quality.available_weight,
        r.data_quality.total_weight
    );
    println!(
        "sum of contributions check: {:.4} (should equal the composite {:.4})",
        r.indicators
            .iter()
            .filter_map(|i| i.contribution)
            .sum::<f64>(),
        r.composite
    );
    let _ = obs;
}
