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
    /// Scan the cohort's 10-Ks for circular-financing disclosure edges, by naming a
    /// counterparty. Networked. Reports SILENCE separately from a zero, because a
    /// counterparty named nowhere is a coverage limit, not a clean bill of health.
    Circular {
        /// Search window start (YYYY-MM-DD).
        #[arg(long, default_value = "2024-01-01")]
        start: String,
        /// Search window end (YYYY-MM-DD).
        #[arg(long, default_value = "2026-09-20")]
        end: String,
        /// Pause between requests, in milliseconds. EDGAR rate-limits.
        #[arg(long, default_value_t = 200)]
        pause_ms: u64,
    },
    /// Survey FUND holdings of private AI companies from NPORT-P filings. Networked.
    ///
    /// This is the investor side of the circular loop with dollar values attached. It is a
    /// LOWER BOUND on institutional exposure: only US-registered funds file N-PORT, so
    /// sovereign funds, family offices and the strategic corporate investors are absent.
    Holdings {
        /// Search window start (YYYY-MM-DD).
        #[arg(long, default_value = "2025-01-01")]
        start: String,
        /// Search window end (YYYY-MM-DD).
        #[arg(long, default_value = "2026-09-20")]
        end: String,
        /// How many filings per name to fetch and parse.
        #[arg(long, default_value_t = 3)]
        per_name: usize,
        /// Pause between requests, in milliseconds.
        #[arg(long, default_value_t = 250)]
        pause_ms: u64,
    },
    /// Probe each source and print its health.
    Sources,
    /// Attribute the composite's recorded history to MODEL changes versus MARKET movement.
    ///
    /// Exists because a composite that can be raised by editing the model, and that reports no
    /// distinction between that and a genuine market move, will be misread by default.
    Drift {
        /// Directory holding the run archive.
        #[arg(long, default_value = "data/history")]
        history_dir: PathBuf,
    },
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
    /// Report which recent days have NO archive row.
    ///
    /// A daily series with a hole in it is a SILENT failure: nothing errors, the
    /// trend simply spans a gap as though the missing days never existed. Measured
    /// 2026-09-24: the host was off across the 13:30 timer slot and `Persistent=true`
    /// did not fire a catch-up run, so that day is absent with no trace anywhere in
    /// the output. This makes a missed run a visible gap instead.
    Doctor {
        /// Directory holding the run archive.
        #[arg(long, default_value = "data/history")]
        history_dir: PathBuf,
        /// How many days back to check.
        #[arg(long, default_value_t = 30)]
        days: i64,
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
        Cmd::Holdings {
            start,
            end,
            per_name,
            pause_ms,
        } => {
            let fetcher = bubble_watch::http::Fetcher::new(cli.offline, cache.clone());
            let mut all: Vec<bubble_watch::sources::nport::FundHolding> = Vec::new();
            let mut errors: Vec<String> = Vec::new();
            let mut unfound: Vec<String> = Vec::new();
            let mut unparsed: Vec<String> = Vec::new();

            for name in bubble_watch::sources::nport::PRIVATE_AI_NAMES {
                match bubble_watch::sources::nport::find_filings(
                    &fetcher, name, start, end, *per_name,
                ) {
                    Ok(filings) if filings.is_empty() => unfound.push(name.to_string()),
                    Ok(filings) => {
                        for (filer_cik, date, id) in filings {
                            let (filer, cik) = filer_cik
                                .split_once('|')
                                .unwrap_or((filer_cik.as_str(), ""));
                            let Some(url) = bubble_watch::sources::nport::document_url(cik, &id)
                            else {
                                unparsed.push(format!("{}: unparseable id {}", name, id));
                                continue;
                            };
                            match fetcher.get(
                                &url,
                                "bubble-watch/0.1 (research; contact: research@example.invalid)",
                            ) {
                                Ok(xml) => {
                                    let (mut hs, dropped) =
                                        bubble_watch::sources::nport::parse_holdings(
                                            &xml,
                                            filer,
                                            cik,
                                            &date,
                                            &[*name],
                                        );
                                    if dropped > 0 {
                                        unparsed.push(format!(
                                            "{}: {} holding(s) in {} dropped for a missing valUSD",
                                            name, dropped, filer
                                        ));
                                    }
                                    all.append(&mut hs);
                                }
                                Err(e) => errors.push(format!("{} {}: {}", name, url, e)),
                            }
                            std::thread::sleep(std::time::Duration::from_millis(*pause_ms));
                        }
                    }
                    Err(e) => errors.push(format!("{}: {}", name, e)),
                }
            }

            println!("PRIVATE-AI HOLDINGS IN US-REGISTERED FUNDS (from NPORT-P)");
            println!("{}", "=".repeat(72));
            all.sort_by(|a, b| {
                b.val_usd
                    .partial_cmp(&a.val_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            println!(
                "Holdings parsed: {}. Query errors: {}.",
                all.len(),
                errors.len()
            );
            println!();
            if !all.is_empty() {
                println!(
                    "{:>16}  {:<11} {:<34} {}",
                    "VALUE (USD)", "HOLDING", "FILED NAME", "L3?"
                );
                for h in &all {
                    println!(
                        "{:>16.0}  {:<11} {:<34} {}",
                        h.val_usd,
                        h.holding,
                        h.filed_name.chars().take(34).collect::<String>(),
                        if h.fair_value_level == Some(3) {
                            "LEVEL 3"
                        } else {
                            ""
                        }
                    );
                }
                let l3 = all.iter().filter(|h| h.fair_value_level == Some(3)).count();
                println!();
                println!(
                    "{} of {} holdings are FAIR-VALUE LEVEL 3 — valued from UNOBSERVABLE inputs,",
                    l3,
                    all.len()
                );
                println!(
                    "so the dollar figures are the filer's own estimate, not a transaction price."
                );
            }
            if !unfound.is_empty() {
                println!();
                println!("NAMES WITH NO HOLDING FOUND (a coverage limit, not a clean result):");
                for n in &unfound {
                    println!(
                        "  ! {} — only US-registered funds file N-PORT. Sovereign funds,",
                        n
                    );
                    println!(
                        "    family offices and the STRATEGIC corporate investors whose stakes"
                    );
                    println!("    are the actual circularity are NOT covered by this source.");
                }
            }
            if !unparsed.is_empty() {
                println!();
                println!("DROPPED / UNPARSABLE (reported, never zeroed):");
                for u in &unparsed {
                    println!("  ! {}", u);
                }
            }
            if !errors.is_empty() {
                println!();
                println!("QUERY ERRORS:");
                for e in &errors {
                    println!("  ! {}", e);
                }
            }
            Ok(())
        }
        Cmd::Circular {
            start,
            end,
            pause_ms,
        } => {
            let fetcher = bubble_watch::http::Fetcher::new(cli.offline, cache.clone());
            let res = bubble_watch::sources::circularity::scan(&fetcher, start, end, *pause_ms);
            println!("CIRCULAR-FINANCING DISCLOSURE SCAN");
            println!("{}", "=".repeat(72));
            println!(
                "Filers scanned: {}. Edges found: {}. Failed queries: {}.",
                res.filers_scanned,
                res.edges.len(),
                res.errors.len()
            );
            println!();
            if res.edges.is_empty() {
                println!("No disclosure edges found. This is NOT a clean result — see the");
                println!("silence notes below before reading anything into it.");
            } else {
                println!(
                    "{:<7} {:<12} {:>5}  {}",
                    "FILER", "COUNTERPARTY", "HITS", "LATEST FILINGS"
                );
                for e in &res.edges {
                    println!(
                        "{:<7} {:<12} {:>5}  {}",
                        e.filer,
                        e.counterparty,
                        e.hits,
                        e.dates
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    if let Some(p) = &e.verified_passage {
                        println!("        verified: {}", p);
                    }
                    match &e.magnitude {
                        Some(m) => println!("        magnitude: {}", m),
                        None => println!("        magnitude: not disclosed / not yet read"),
                    }
                }
            }
            if !res.silent.is_empty() {
                println!();
                println!("COUNTERPARTIES NAMED NOWHERE (read as a coverage limit, not as safety):");
                for s in &res.silent {
                    println!("  ! {}", s);
                }
            }
            if !res.errors.is_empty() {
                println!();
                println!("FAILED QUERIES (reported, not treated as zeros):");
                for e in &res.errors {
                    println!("  ! {}", e);
                }
            }
            Ok(())
        }
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
        Cmd::Drift { history_dir } => {
            let (archive, warnings) = bubble_watch::history::load(history_dir);
            let d = bubble_watch::drift::attribute_with_cfg(&archive, &cfg);
            println!("COMPOSITE HISTORY — MODEL vs MARKET");
            println!("{}", "=".repeat(72));
            println!("{}", d.statement());
            println!();
            println!(
                "{:<8} {:>6} {:>8} {:>8} {:>8}",
                "METHOD", "RUNS", "FIRST", "LAST", "RANGE"
            );
            for e in &d.epochs {
                println!(
                    "{:<8} {:>6} {:>8.1} {:>8.1} {:>8.2}",
                    e.methodology,
                    e.runs,
                    e.first_composite,
                    e.last_composite,
                    e.market_range()
                );
            }
            println!();
            println!(
                "Within-methodology range is the ONLY part attributable to the market. Every other \
                 movement in this table is the instrument changing."
            );
            if d.phase_changes_at_model_boundary > 0 {
                println!();
                println!(
                    "  ! The PHASE LABEL crossed at a model boundary ('{}' -> '{}'), so the TIMING \
                     OVERLAY also changed without the market moving: 'early' quotes 30-72 months \
                     and 'mid' quotes 15-42. A reader who saw the earlier range and the later one \
                     would reasonably conclude the window had halved. It had not.",
                    d.phase_first, d.phase_last
                );
            }
            for w in &warnings {
                println!("  ! {}", w);
            }
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
        Cmd::Doctor { history_dir, days } => {
            let (rows, problems) = bubble_watch::history::load(history_dir);
            for p in &problems {
                eprintln!("archive problem: {p}");
            }
            let today = bubble_watch::now_date();
            let missing = bubble_watch::history::missing_dates(&rows, &today, *days);

            println!(
                "archive: {} row(s) across {} distinct date(s), checked {} day(s) to {}",
                rows.len(),
                rows.iter()
                    .map(|r| r.date.as_str())
                    .collect::<std::collections::HashSet<_>>()
                    .len(),
                days,
                today
            );

            if missing.is_empty() {
                println!("no gaps: every day in the window has at least one run recorded");
                return Ok(());
            }

            // A gap is reported as a FAILURE, not a note: a daily instrument whose
            // history has holes silently compares across them, and that is the whole
            // reason this command exists.
            println!("\n{} GAP(S) — days with no recorded run:", missing.len());
            for d in &missing {
                println!("  {d}");
            }
            println!(
                "\nA gap is not cosmetic: `trend` spans it as though the missing days never \
                 existed, so direction of travel is computed across an interval nothing was \
                 measured in. Check whether the host was off across the timer slot (measured \
                 2026-09-24: the host booted at 13:55, after the 13:30 run, and Persistent=true \
                 did NOT fire a catch-up)."
            );
            std::process::exit(1);
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

    // Declared judgment. Printed after the falsifiers: the reader has seen what would
    // disprove the thesis, and now sees what the tool DOES NOT claim to measure.
    if !r.judgments.is_empty() {
        let against = bubble_watch::subjective::against_count(&r.judgments);
        let jonly = bubble_watch::subjective::judgment_only_count(&r.judgments);
        println!();
        println!(
            "DECLARED JUDGMENT AND UNSCORED EVIDENCE ({} entries; {} bear against the thesis, {} are judgment only)",
            r.judgments.len(), against, jonly
        );
        println!("  NOT part of the composite. A belief averaged into a measurement would destroy");
        println!("  the distinction between the two, so judgment is declared here instead.");
        for j in &r.judgments {
            println!(
                "  [{} / {} / {}] {}",
                j.bears.as_str(),
                j.basis.as_str(),
                j.confidence.as_str(),
                j.claim
            );
            println!("      evidence : {}", j.evidence);
            println!("      falsifier: {}", j.must_not);
        }
    }

    // Falsification tests. Printed BEFORE the exposure table and after the score,
    // because the reader should see what would prove the tool wrong before they
    // see a number that might otherwise only reassure them one way.
    if !r.falsifiers.is_empty() {
        let against = bubble_watch::falsifiers::counter_evidence_count(&r.falsifiers);
        println!();
        println!(
            "FALSIFICATION TESTS - what would show the bubble thesis is WRONG ({} of {} read as counter-evidence)",
            against,
            r.falsifiers.len()
        );
        for f in &r.falsifiers {
            let mark = match f.verdict {
                bubble_watch::falsifiers::Verdict::CounterEvidence => "AGAINST ",
                bubble_watch::falsifiers::Verdict::ConsistentWithBubble => "for     ",
                bubble_watch::falsifiers::Verdict::Uninformative => "n/a     ",
            };
            println!("  [{}] {}", mark, f.question);
            println!("           reading: {}", f.reading);
            println!("           verdict: {}", f.verdict.as_str());
        }
        println!(
            "  These are NOT part of the composite. Averaging evidence for and against into one"
        );
        println!("  number would merge opposite meanings, so they are reported beside it.");
    }

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
